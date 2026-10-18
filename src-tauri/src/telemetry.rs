//! YouTube Data API v3 client: an ETag cache, a local estimate of the daily quota, and error
//! messages meant for people.

use crate::credentials::FileVault;
use crate::security::{
    sanitize_display_text, sanitize_sensitive_text, validate_api_key_format,
    validate_safe_identifier, validate_trusted_image_url, CredentialProvider,
};
use reqwest::header::{ETAG, IF_NONE_MATCH};
use reqwest::{Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

pub const YOUTUBE_DAILY_QUOTA_LIMIT: u32 = 10_000;

/// Each of `channels.list`, `playlistItems.list` and `videos.list` costs one unit.
const QUOTA_COST_PER_CALL: u32 = 1;
/// Google recommends this header over the `key` query parameter, which would put the key in URLs.
const API_KEY_HEADER: &str = "x-goog-api-key";
const RECENT_UPLOADS: usize = 10;
/// Short enough to fit on the dashboard's one-line status row.
const QUOTA_MESSAGE: &str = "Daily quota used up. It resets at midnight Pacific.";

/// Day index in Pacific time (fixed UTC-8, an hour off during daylight saving time), because the
/// YouTube quota resets at midnight Pacific.
pub fn pacific_day_at(unix_secs: u64) -> u64 {
    unix_secs.saturating_sub(8 * 3600) / 86_400
}

fn unix_now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryEndpoints {
    pub youtube_base_url: String,
}

impl Default for TelemetryEndpoints {
    fn default() -> Self {
        Self {
            youtube_base_url: "https://www.googleapis.com/youtube/v3".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeMetrics {
    pub channel_id: String,
    pub channel_title: Option<String>,
    pub custom_url: Option<String>,
    pub avatar_url: Option<String>,
    pub country: Option<String>,
    pub published_at: Option<String>,
    pub subscriber_count: u64,
    pub video_count: u64,
    pub view_count: u64,
    /// Likes and comments below are totals over the last 10 uploads, not lifetime channel totals.
    pub like_count: u64,
    pub comment_count: u64,
    pub latest_video_title: Option<String>,
    pub latest_video_thumbnail: Option<String>,
    pub latest_video_published_at: Option<String>,
    pub latest_video_views: u64,
    pub latest_video_likes: u64,
    pub latest_video_comments: u64,
    // Request diagnostics for the most recent refresh.
    pub etag: String,
    pub from_etag_cache: bool,
    pub quota_cost_last_call: u32,
    pub total_quota_used_today: u32,
    pub etag_304_hits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySnapshot {
    pub youtube: Option<YouTubeMetrics>,
    pub errors: HashMap<String, String>,
    pub last_updated_unix_ms: u64,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    etag: String,
    metrics: YouTubeMetrics,
}

#[derive(Debug, Default)]
struct EngineState {
    cache: HashMap<String, CacheEntry>,
    quota_used_today: u32,
    /// Pacific day index of the current quota window.
    quota_day: u64,
    not_modified_hits: u32,
    last_snapshot: TelemetrySnapshot,
}

#[derive(Clone)]
pub struct TelemetryEngine {
    http: reqwest::Client,
    endpoints: TelemetryEndpoints,
    state: Arc<RwLock<EngineState>>,
}

impl Default for TelemetryEngine {
    fn default() -> Self {
        Self::new(TelemetryEndpoints::default())
    }
}

impl TelemetryEngine {
    pub fn new(endpoints: TelemetryEndpoints) -> Self {
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent(concat!(
                "GrowthNotch/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/zribibecher/growth-notch)"
            ))
            .build()
            .expect("failed to build reqwest HTTP client");

        Self {
            http,
            endpoints,
            state: Arc::new(RwLock::new(EngineState::default())),
        }
    }

    /// Refreshes the channel using the credentials in the vault. With no credentials there is
    /// nothing to show and no error. After a failure the previous numbers stay on screen.
    pub async fn poll_all(&self, vault: &FileVault) -> TelemetrySnapshot {
        let mut errors = HashMap::new();

        let credentials = (
            vault.get_secret(CredentialProvider::YoutubeChannelId),
            vault.get_secret(CredentialProvider::YoutubeApiKey),
        );
        let youtube = if let (Ok(Some(channel_id)), Ok(Some(api_key))) = credentials {
            match self
                .fetch_youtube(channel_id.expose_secret(), api_key.expose_secret())
                .await
            {
                Ok(metrics) => Some(metrics),
                Err(err) => {
                    errors.insert("youtube".to_string(), sanitize_sensitive_text(&err));
                    self.state.read().await.last_snapshot.youtube.clone()
                }
            }
        } else {
            None
        };

        let snapshot = TelemetrySnapshot {
            youtube,
            errors,
            last_updated_unix_ms: unix_now().as_millis() as u64,
        };
        self.state.write().await.last_snapshot = snapshot.clone();
        snapshot
    }

    /// Fetches the channel, and when it changed, the last 10 uploads. Sends `If-None-Match` with
    /// the previous ETag, so an unchanged channel costs a tiny `304` and skips the two upload calls.
    pub async fn fetch_youtube(
        &self,
        channel_id: &str,
        api_key: &str,
    ) -> Result<YouTubeMetrics, String> {
        let channel_id = validate_safe_identifier(channel_id, "YouTube channel ID")?;
        let api_key = validate_api_key_format(api_key)?;

        let cached = self.begin_request(channel_id).await?;
        let response = self
            .get(
                "channels",
                api_key,
                &[
                    ("part", "statistics,snippet,contentDetails"),
                    ("id", channel_id),
                ],
                cached.as_ref().map(|entry| entry.etag.as_str()),
            )
            .await?;

        if response.status() == StatusCode::NOT_MODIFIED {
            return self.serve_cached(channel_id, cached).await;
        }

        let etag = response
            .headers()
            .get(ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let list: ChannelList = parse_success(response).await?;
        let etag = etag.or(list.etag).unwrap_or_default();
        let item = list
            .items
            .into_iter()
            .next()
            .ok_or_else(|| "No channel found for that ID. Check it in Setup.".to_string())?;

        let uploads_playlist = uploads_playlist_id(&item, channel_id);
        let engagement = match &uploads_playlist {
            Some(playlist_id) => self
                .fetch_recent_engagement(api_key, playlist_id)
                .await
                .unwrap_or_default(),
            None => VideoEngagement::default(),
        };
        // channels.list always; playlistItems.list and videos.list when uploads were queried.
        let cost = if uploads_playlist.is_some() {
            3 * QUOTA_COST_PER_CALL
        } else {
            QUOTA_COST_PER_CALL
        };

        let mut state = self.state.write().await;
        state.quota_used_today += cost;
        let mut metrics = build_metrics(channel_id, item, engagement);
        metrics.etag = etag.clone();
        metrics.quota_cost_last_call = cost;
        metrics.total_quota_used_today = state.quota_used_today;
        metrics.etag_304_hits = state.not_modified_hits;
        if !etag.is_empty() {
            state.cache.insert(
                channel_id.to_string(),
                CacheEntry {
                    etag,
                    metrics: metrics.clone(),
                },
            );
        }
        Ok(metrics)
    }

    /// Rolls the quota window over at midnight Pacific, refuses to continue once the daily
    /// limit is spent, and returns the cached entry for this channel, if any.
    async fn begin_request(&self, channel_id: &str) -> Result<Option<CacheEntry>, String> {
        let mut state = self.state.write().await;
        let today = pacific_day_at(unix_now().as_secs());
        if state.quota_day != today {
            state.quota_day = today;
            state.quota_used_today = 0;
        }
        if state.quota_used_today >= YOUTUBE_DAILY_QUOTA_LIMIT {
            return Err(QUOTA_MESSAGE.to_string());
        }
        Ok(state.cache.get(channel_id).cloned())
    }

    async fn serve_cached(
        &self,
        channel_id: &str,
        cached: Option<CacheEntry>,
    ) -> Result<YouTubeMetrics, String> {
        let entry = cached
            .ok_or_else(|| "YouTube answered 304 but nothing was cached to show".to_string())?;
        let mut state = self.state.write().await;
        state.not_modified_hits += 1;
        // Google does not say a 304 is free, so it counts as the one unit the channel call costs.
        state.quota_used_today += QUOTA_COST_PER_CALL;

        let mut metrics = entry.metrics;
        metrics.from_etag_cache = true;
        metrics.quota_cost_last_call = QUOTA_COST_PER_CALL;
        metrics.total_quota_used_today = state.quota_used_today;
        metrics.etag_304_hits = state.not_modified_hits;
        state.cache.insert(
            channel_id.to_string(),
            CacheEntry {
                etag: entry.etag,
                metrics: metrics.clone(),
            },
        );
        Ok(metrics)
    }

    /// Reads the last 10 uploads (`playlistItems.list`, then `videos.list`). Returns `None` on
    /// any failure, so the channel numbers still show without the upload details.
    async fn fetch_recent_engagement(
        &self,
        api_key: &str,
        playlist_id: &str,
    ) -> Option<VideoEngagement> {
        let max_results = RECENT_UPLOADS.to_string();
        let playlist: PlaylistItems = parse_success(
            self.get(
                "playlistItems",
                api_key,
                &[
                    ("part", "contentDetails"),
                    ("playlistId", playlist_id),
                    ("maxResults", max_results.as_str()),
                ],
                None,
            )
            .await
            .ok()?,
        )
        .await
        .ok()?;

        let video_ids: Vec<&str> = playlist
            .items
            .iter()
            .filter_map(|item| item.content_details.as_ref()?.video_id.as_deref())
            .filter(|id| validate_safe_identifier(id, "YouTube video ID").is_ok())
            .take(RECENT_UPLOADS)
            .collect();
        if video_ids.is_empty() {
            return None;
        }

        let ids = video_ids.join(",");
        let videos: VideoList = parse_success(
            self.get(
                "videos",
                api_key,
                &[("part", "statistics,snippet"), ("id", ids.as_str())],
                None,
            )
            .await
            .ok()?,
        )
        .await
        .ok()?;

        let mut engagement = VideoEngagement::default();
        for (index, video) in videos.items.iter().enumerate() {
            let stats = video.statistics.clone().unwrap_or_default();
            let (views, likes, comments) = (
                count(&stats.view_count),
                count(&stats.like_count),
                count(&stats.comment_count),
            );
            engagement.total_views = engagement.total_views.saturating_add(views);
            engagement.total_likes = engagement.total_likes.saturating_add(likes);
            engagement.total_comments = engagement.total_comments.saturating_add(comments);

            if index == 0 {
                let snippet = video.snippet.clone().unwrap_or_default();
                engagement.latest_title = clean(Some(snippet.title.as_str()), 160);
                engagement.latest_thumbnail = snippet
                    .thumbnails
                    .as_ref()
                    .and_then(|thumbnails| thumbnails.first_trusted(false));
                engagement.latest_published_at = clean(snippet.published_at.as_deref(), 40);
                engagement.latest_views = views;
                engagement.latest_likes = likes;
                engagement.latest_comments = comments;
            }
        }
        Some(engagement)
    }

    /// One GET against the API with the key in a header. Transport errors never include the URL.
    async fn get(
        &self,
        path: &str,
        api_key: &str,
        query: &[(&str, &str)],
        if_none_match: Option<&str>,
    ) -> Result<Response, String> {
        let base = self.endpoints.youtube_base_url.trim_end_matches('/');
        let mut request = self
            .http
            .get(format!("{base}/{path}"))
            .header(API_KEY_HEADER, api_key)
            .query(query);
        if let Some(etag) = if_none_match {
            request = request.header(IF_NONE_MATCH, etag);
        }
        request.send().await.map_err(|err| {
            if err.is_timeout() {
                "YouTube took too long to answer.".to_string()
            } else {
                "Can't reach YouTube. Check your connection.".to_string()
            }
        })
    }
}

/// Decodes a successful response, or turns an error response into a readable message.
async fn parse_success<T: DeserializeOwned>(response: Response) -> Result<T, String> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(describe_error(status, &body));
    }
    response
        .json()
        .await
        .map_err(|_| "YouTube sent a response this app could not read.".to_string())
}

/// Turns a Google API error into one short sentence about what to do. Nothing from the response
/// body is passed through unfiltered.
fn describe_error(status: StatusCode, body: &str) -> String {
    let json: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let error = &json["error"];
    let message = error["message"].as_str().unwrap_or_default();
    let reasons: Vec<&str> = error["errors"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(error["details"].as_array().into_iter().flatten())
        .filter_map(|entry| entry["reason"].as_str())
        .collect();
    let has_reason = |names: &[&str]| reasons.iter().any(|reason| names.contains(reason));

    let text = if has_reason(&["API_KEY_INVALID"]) || message.contains("API key not valid") {
        "YouTube rejected the API key. Paste it again in Setup.".to_string()
    } else if has_reason(&[
        "quotaExceeded",
        "dailyLimitExceeded",
        "rateLimitExceeded",
        "userRateLimitExceeded",
    ]) {
        QUOTA_MESSAGE.to_string()
    } else if has_reason(&[
        "accessNotConfigured",
        "SERVICE_DISABLED",
        "API_KEY_SERVICE_BLOCKED",
    ]) {
        "Turn on YouTube Data API v3 for this key in Google Cloud.".to_string()
    } else if status == StatusCode::FORBIDDEN {
        "YouTube refused the request. Check this key's restrictions.".to_string()
    } else if status == StatusCode::NOT_FOUND {
        "YouTube could not find that channel.".to_string()
    } else if status.is_server_error() {
        format!(
            "YouTube is having trouble (HTTP {}). It will try again.",
            status.as_u16()
        )
    } else {
        let detail = clean(Some(message), 120).unwrap_or_default();
        format!("YouTube returned HTTP {}. {detail}", status.as_u16())
            .trim_end()
            .to_string()
    };
    sanitize_sensitive_text(&text)
}

fn uploads_playlist_id(item: &ChannelItem, channel_id: &str) -> Option<String> {
    let from_api = item
        .content_details
        .as_ref()
        .and_then(|details| details.related_playlists.as_ref())
        .and_then(|playlists| playlists.uploads.clone());
    // A channel's uploads playlist is the channel ID with "UC" swapped for "UU".
    let derived = || {
        channel_id
            .strip_prefix("UC")
            .filter(|rest| !rest.is_empty())
            .map(|rest| format!("UU{rest}"))
    };
    from_api
        .or_else(derived)
        .filter(|id| validate_safe_identifier(id, "YouTube uploads playlist ID").is_ok())
}

fn build_metrics(
    channel_id: &str,
    item: ChannelItem,
    engagement: VideoEngagement,
) -> YouTubeMetrics {
    let snippet = item.snippet.unwrap_or_default();
    let statistics = item.statistics;
    YouTubeMetrics {
        channel_id: channel_id.to_string(),
        channel_title: clean(Some(snippet.title.as_str()), 120),
        custom_url: clean(snippet.custom_url.as_deref(), 80),
        avatar_url: snippet
            .thumbnails
            .as_ref()
            .and_then(|thumbnails| thumbnails.first_trusted(true)),
        country: clean(snippet.country.as_deref(), 16),
        published_at: clean(snippet.published_at.as_deref(), 40),
        subscriber_count: count(&statistics.subscriber_count),
        video_count: count(&statistics.video_count),
        view_count: count(&statistics.view_count).max(engagement.total_views),
        like_count: engagement.total_likes,
        comment_count: engagement.total_comments,
        latest_video_title: engagement.latest_title,
        latest_video_thumbnail: engagement.latest_thumbnail,
        latest_video_published_at: engagement.latest_published_at,
        latest_video_views: engagement.latest_views,
        latest_video_likes: engagement.latest_likes,
        latest_video_comments: engagement.latest_comments,
        etag: String::new(),
        from_etag_cache: false,
        quota_cost_last_call: 0,
        total_quota_used_today: 0,
        etag_304_hits: 0,
    }
}

/// YouTube sends every count as a string. A missing or unreadable one counts as zero.
fn count(value: &Option<String>) -> u64 {
    value
        .as_deref()
        .and_then(|text| text.parse().ok())
        .unwrap_or(0)
}

/// Strips control and bidirectional characters, trims, truncates, and drops empty strings.
fn clean(value: Option<&str>, max_chars: usize) -> Option<String> {
    value
        .map(|text| sanitize_display_text(text, max_chars))
        .filter(|text| !text.is_empty())
}

#[derive(Debug, Default)]
struct VideoEngagement {
    total_views: u64,
    total_likes: u64,
    total_comments: u64,
    latest_title: Option<String>,
    latest_thumbnail: Option<String>,
    latest_published_at: Option<String>,
    latest_views: u64,
    latest_likes: u64,
    latest_comments: u64,
}

#[derive(Debug, Deserialize)]
struct ChannelList {
    etag: Option<String>,
    #[serde(default)]
    items: Vec<ChannelItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChannelItem {
    snippet: Option<Snippet>,
    #[serde(default)]
    statistics: Statistics,
    content_details: Option<ContentDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentDetails {
    related_playlists: Option<RelatedPlaylists>,
}

#[derive(Debug, Deserialize)]
struct RelatedPlaylists {
    uploads: Option<String>,
}

/// Used for both channels and videos, which share these fields.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snippet {
    #[serde(default)]
    title: String,
    custom_url: Option<String>,
    country: Option<String>,
    published_at: Option<String>,
    thumbnails: Option<Thumbnails>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Statistics {
    subscriber_count: Option<String>,
    video_count: Option<String>,
    view_count: Option<String>,
    like_count: Option<String>,
    comment_count: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Thumbnails {
    default: Option<Thumbnail>,
    medium: Option<Thumbnail>,
    high: Option<Thumbnail>,
}

#[derive(Debug, Clone, Deserialize)]
struct Thumbnail {
    url: String,
}

impl Thumbnails {
    /// The first thumbnail on a trusted image host, biggest first or a medium one first.
    fn first_trusted(&self, prefer_large: bool) -> Option<String> {
        let ordered = if prefer_large {
            [&self.high, &self.medium, &self.default]
        } else {
            [&self.medium, &self.high, &self.default]
        };
        ordered
            .into_iter()
            .flatten()
            .find_map(|thumbnail| validate_trusted_image_url(&thumbnail.url))
    }
}

#[derive(Debug, Deserialize)]
struct PlaylistItems {
    #[serde(default)]
    items: Vec<PlaylistItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistItem {
    content_details: Option<PlaylistItemDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistItemDetails {
    video_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VideoList {
    #[serde(default)]
    items: Vec<VideoItem>,
}

#[derive(Debug, Deserialize)]
struct VideoItem {
    snippet: Option<Snippet>,
    statistics: Option<Statistics>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_message_for_a_rejected_key() {
        let body = r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","errors":[{"reason":"badRequest"}],"details":[{"reason":"API_KEY_INVALID"}]}}"#;
        let text = describe_error(StatusCode::BAD_REQUEST, body);
        assert_eq!(
            text,
            "YouTube rejected the API key. Paste it again in Setup."
        );
    }

    #[test]
    fn friendly_message_when_the_quota_is_spent() {
        let body =
            r#"{"error":{"code":403,"message":"quota","errors":[{"reason":"quotaExceeded"}]}}"#;
        assert_eq!(describe_error(StatusCode::FORBIDDEN, body), QUOTA_MESSAGE);
    }

    #[test]
    fn friendly_message_when_the_api_is_not_enabled() {
        let body = r#"{"error":{"code":403,"errors":[{"reason":"accessNotConfigured"}]}}"#;
        assert!(
            describe_error(StatusCode::FORBIDDEN, body).starts_with("Turn on YouTube Data API v3")
        );
    }

    #[test]
    fn unknown_errors_never_pass_the_raw_body_through() {
        let body = "<html>secret key=unit_test_secret_123 body</html>";
        let text = describe_error(StatusCode::IM_A_TEAPOT, body);
        assert_eq!(text, "YouTube returned HTTP 418.");
        assert!(!text.contains("secret"));
    }

    #[test]
    fn upstream_messages_are_scrubbed() {
        let body = r#"{"error":{"message":"bad key=unit_test_secret_456 here"}}"#;
        let text = describe_error(StatusCode::UNAUTHORIZED, body);
        assert!(!text.contains("unit_test_secret_456"));
        assert!(text.contains("[REDACTED]"));
    }

    #[test]
    fn uploads_playlist_comes_from_the_api_or_the_channel_id() {
        let item = ChannelItem {
            snippet: None,
            statistics: Statistics::default(),
            content_details: None,
        };
        assert_eq!(
            uploads_playlist_id(&item, "UCabcdefghijklmnopqrstuv").as_deref(),
            Some("UUabcdefghijklmnopqrstuv")
        );
        assert_eq!(uploads_playlist_id(&item, "not-a-channel"), None);
    }

    #[test]
    fn counts_default_to_zero() {
        assert_eq!(count(&Some("1234".into())), 1234);
        assert_eq!(count(&Some("x".into())), 0);
        assert_eq!(count(&None), 0);
    }
}
