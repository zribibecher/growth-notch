mod common;

use common::{channel_json, MockServer, Response, API_KEY, CHANNEL_ID};
use growth_notch_lib::credentials::FileVault;
use growth_notch_lib::security::CredentialProvider;
use growth_notch_lib::telemetry::{pacific_day_at, TelemetryEndpoints, TelemetryEngine};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn engine_for(server: &MockServer) -> TelemetryEngine {
    TelemetryEngine::new(TelemetryEndpoints {
        youtube_base_url: server.base_url.clone(),
    })
}

fn vault_with_credentials() -> FileVault {
    let vault = FileVault::in_memory();
    vault
        .set_secret(CredentialProvider::YoutubeChannelId, CHANNEL_ID)
        .unwrap();
    vault
        .set_secret(CredentialProvider::YoutubeApiKey, API_KEY)
        .unwrap();
    vault
}

#[tokio::test]
async fn the_first_refresh_reads_the_channel_and_its_last_uploads() {
    let server = MockServer::youtube().await;
    let metrics = engine_for(&server)
        .fetch_youtube(CHANNEL_ID, API_KEY)
        .await
        .unwrap();

    assert_eq!(metrics.channel_title.as_deref(), Some("Tech Untangled"));
    assert_eq!(metrics.custom_url.as_deref(), Some("@techuntangled"));
    assert_eq!(metrics.subscriber_count, 142_500);
    assert_eq!(metrics.video_count, 94);
    assert_eq!(metrics.view_count, 8_910_200);
    // Likes and comments are summed over the uploads that came back.
    assert_eq!(metrics.like_count, 4_100 + 6_200);
    assert_eq!(metrics.comment_count, 380 + 520);
    assert_eq!(
        metrics.latest_video_title.as_deref(),
        Some("Building a Local Rust Notch Widget")
    );
    assert_eq!(
        metrics.latest_video_published_at.as_deref(),
        Some("2026-10-01T09:30:00Z")
    );
    assert_eq!(metrics.latest_video_views, 52_300);
    assert_eq!(metrics.latest_video_likes, 4_100);
    assert_eq!(metrics.latest_video_comments, 380);

    // channels.list + playlistItems.list + videos.list, one unit each.
    assert!(!metrics.from_etag_cache);
    assert_eq!(metrics.quota_cost_last_call, 3);
    assert_eq!(metrics.total_quota_used_today, 3);
}

#[tokio::test]
async fn images_only_come_from_youtube_hosts() {
    let server = MockServer::youtube().await;
    let metrics = engine_for(&server)
        .fetch_youtube(CHANNEL_ID, API_KEY)
        .await
        .unwrap();

    // Both fixtures list an evil.example.com image first; it must be skipped.
    assert_eq!(
        metrics.avatar_url.as_deref(),
        Some("https://yt3.ggpht.com/avatar=s800")
    );
    assert_eq!(
        metrics.latest_video_thumbnail.as_deref(),
        Some("https://i.ytimg.com/vi/vid_latest_1/mqdefault.jpg")
    );
}

#[tokio::test]
async fn an_unchanged_channel_is_served_from_the_etag_cache() {
    let server = MockServer::youtube().await;
    let engine = engine_for(&server);
    engine.fetch_youtube(CHANNEL_ID, API_KEY).await.unwrap();
    let again = engine.fetch_youtube(CHANNEL_ID, API_KEY).await.unwrap();

    assert!(again.from_etag_cache);
    assert_eq!(again.subscriber_count, 142_500);
    assert_eq!(again.like_count, 10_300);
    assert_eq!(again.etag_304_hits, 1);
    // A 304 still counts as the one unit of the channel call: Google does not say it is free.
    assert_eq!(again.quota_cost_last_call, 1);
    assert_eq!(again.total_quota_used_today, 4);

    // The second round skipped the two upload calls.
    assert_eq!(server.hits("channels"), 2);
    assert_eq!(server.hits("playlistItems"), 1);
    assert_eq!(server.hits("videos"), 1);
}

#[tokio::test]
async fn the_api_key_travels_in_a_header_and_never_in_a_url() {
    let server = MockServer::youtube().await;
    engine_for(&server)
        .fetch_youtube(CHANNEL_ID, API_KEY)
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    for request in &requests {
        assert_eq!(
            request.headers.get("x-goog-api-key").map(String::as_str),
            Some(API_KEY),
            "{} is missing the key header",
            request.path
        );
        assert!(!request.path.contains(API_KEY));
        assert!(!request.query.contains(API_KEY));
        assert!(!request.query.to_ascii_lowercase().contains("key="));
        assert!(request.headers["user-agent"].starts_with("GrowthNotch/"));
    }
}

#[tokio::test]
async fn a_refresh_through_the_vault_reports_no_errors() {
    let server = MockServer::youtube().await;
    let snapshot = engine_for(&server)
        .poll_all(&vault_with_credentials())
        .await;

    assert!(snapshot.errors.is_empty(), "{:?}", snapshot.errors);
    assert_eq!(snapshot.youtube.unwrap().subscriber_count, 142_500);
    assert!(snapshot.last_updated_unix_ms > 0);
}

#[tokio::test]
async fn without_credentials_there_is_nothing_to_show_and_nothing_is_sent() {
    let server = MockServer::youtube().await;
    let snapshot = engine_for(&server).poll_all(&FileVault::in_memory()).await;

    assert!(snapshot.youtube.is_none());
    assert!(snapshot.errors.is_empty());
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn after_a_failed_refresh_the_previous_numbers_stay_on_screen() {
    let failing = Arc::new(AtomicBool::new(false));
    let switch = failing.clone();
    let server = MockServer::start(move |request| {
        if switch.load(Ordering::SeqCst) {
            Response::empty(503)
        } else {
            common::youtube_handler(request)
        }
    })
    .await;
    let engine = engine_for(&server);
    let vault = vault_with_credentials();

    let healthy = engine.poll_all(&vault).await;
    assert_eq!(healthy.youtube.as_ref().unwrap().subscriber_count, 142_500);

    failing.store(true, Ordering::SeqCst);
    let degraded = engine.poll_all(&vault).await;
    assert_eq!(degraded.youtube.as_ref().unwrap().subscriber_count, 142_500);
    assert_eq!(
        degraded.errors["youtube"],
        "YouTube is having trouble (HTTP 503). It will try again."
    );
}

#[tokio::test]
async fn error_responses_become_short_sentences_about_what_to_do() {
    let cases = [
        (
            400,
            r#"{"error":{"message":"API key not valid.","details":[{"reason":"API_KEY_INVALID"}]}}"#,
            "YouTube rejected the API key. Paste it again in Setup.",
        ),
        (
            403,
            r#"{"error":{"errors":[{"reason":"quotaExceeded"}]}}"#,
            "Daily quota used up. It resets at midnight Pacific.",
        ),
        (
            403,
            r#"{"error":{"errors":[{"reason":"accessNotConfigured"}]}}"#,
            "Turn on YouTube Data API v3 for this key in Google Cloud.",
        ),
        (
            403,
            r#"{"error":{"message":"Forbidden"}}"#,
            "YouTube refused the request. Check this key's restrictions.",
        ),
        (404, "{}", "YouTube could not find that channel."),
        (
            500,
            "oops",
            "YouTube is having trouble (HTTP 500). It will try again.",
        ),
    ];
    for (status, body, expected) in cases {
        let server = MockServer::start(move |_| Response::json(status, body)).await;
        let error = engine_for(&server)
            .fetch_youtube(CHANNEL_ID, API_KEY)
            .await
            .unwrap_err();
        assert_eq!(error, expected, "HTTP {status}: {body}");
    }
}

#[tokio::test]
async fn a_key_echoed_back_by_the_server_never_reaches_the_ui() {
    let echoed = format!("Bad key={API_KEY} and Bearer fake_test_token_444");
    let body = serde_json::json!({ "error": { "message": echoed } }).to_string();
    let server = MockServer::start(move |_| Response::json(401, &body)).await;

    let snapshot = engine_for(&server)
        .poll_all(&vault_with_credentials())
        .await;
    let error = &snapshot.errors["youtube"];
    assert!(!error.contains(API_KEY), "{error}");
    assert!(!error.contains("fake_test_token_444"), "{error}");
}

#[tokio::test]
async fn text_from_youtube_is_cleaned_before_it_is_shown() {
    let title = format!("Tech\u{202E}gnp.exe\u{202C}\n Channel{}", "x".repeat(300));
    let server = MockServer::start(move |_| Response::json(200, &channel_json(&title))).await;
    let metrics = engine_for(&server)
        .fetch_youtube(CHANNEL_ID, API_KEY)
        .await
        .unwrap();

    let shown = metrics.channel_title.unwrap();
    assert!(shown.starts_with("Techgnp.exe Channelx"), "{shown}");
    assert!(shown.chars().count() <= 120);
    assert!(!shown.contains('\u{202E}') && !shown.contains('\n'));
}

#[tokio::test]
async fn redirects_are_never_followed() {
    let server = MockServer::start(|request| {
        if request.path == "/ssrf-target" {
            Response::json(200, "{}")
        } else {
            Response::empty(302).header("Location", "/ssrf-target")
        }
    })
    .await;

    let result = engine_for(&server).fetch_youtube(CHANNEL_ID, API_KEY).await;
    assert!(result.is_err(), "a 302 must not count as success");
    assert!(
        server.requests().iter().all(|r| r.path != "/ssrf-target"),
        "the client followed a redirect"
    );
}

#[tokio::test]
async fn an_unreachable_server_gives_a_plain_message_without_the_url() {
    let engine = TelemetryEngine::new(TelemetryEndpoints {
        // Nothing listens on port 1.
        youtube_base_url: "http://127.0.0.1:1/youtube".into(),
    });
    let error = engine.fetch_youtube(CHANNEL_ID, API_KEY).await.unwrap_err();
    assert_eq!(error, "Can't reach YouTube. Check your connection.");
}

#[tokio::test]
async fn bad_ids_and_keys_are_refused_before_any_request() {
    let server = MockServer::youtube().await;
    let engine = engine_for(&server);
    for bad_id in ["", "../../v1/admin", "UC123?alt=media&key=leak", "UC 123"] {
        assert!(engine.fetch_youtube(bad_id, API_KEY).await.is_err());
    }
    for bad_key in ["", "key&extra=1", "key with spaces", "key\r\nHeader: x"] {
        assert!(engine.fetch_youtube(CHANNEL_ID, bad_key).await.is_err());
    }
    assert!(server.requests().is_empty());
}

#[test]
fn the_quota_day_rolls_over_at_midnight_pacific() {
    // 2026-01-01 07:59:59 UTC is still Dec 31, 23:59:59 in Pacific time; one second later is the new day.
    let just_before_midnight = 1_767_254_399;
    assert_eq!(
        pacific_day_at(just_before_midnight + 1),
        pacific_day_at(just_before_midnight) + 1
    );
    assert_eq!(
        pacific_day_at(just_before_midnight - 3600),
        pacific_day_at(just_before_midnight)
    );
}
