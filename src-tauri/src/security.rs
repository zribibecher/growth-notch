use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use zeroize::Zeroizing;

/// The credentials the app stores. A strict allowlist keeps an untrusted IPC caller from writing
/// arbitrary entries into the vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialProvider {
    YoutubeApiKey,
    YoutubeChannelId,
}

impl CredentialProvider {
    pub fn from_key(key: &str) -> Result<Self, String> {
        match key.trim().to_ascii_lowercase().as_str() {
            "youtube" | "youtube_api_key" => Ok(Self::YoutubeApiKey),
            "youtube_channel_id" => Ok(Self::YoutubeChannelId),
            other => Err(format!(
                "Unsupported credential provider '{}'. Allowed providers: youtube_api_key, youtube_channel_id",
                sanitize_sensitive_text(other)
            )),
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::YoutubeApiKey => "youtube_api_key",
            Self::YoutubeChannelId => "youtube_channel_id",
        }
    }
}

/// Memory-zeroizing secret wrapper that never prints or serializes its inner value.
#[derive(Clone, PartialEq, Eq)]
pub struct RedactedSecret(Zeroizing<String>);

impl RedactedSecret {
    pub fn new(secret: impl Into<String>) -> Self {
        Self(Zeroizing::new(secret.into()))
    }

    /// Expose the secret only at the point of use: an outbound request or the encrypted vault.
    pub fn expose_secret(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for RedactedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RedactedSecret([REDACTED])")
    }
}

impl fmt::Display for RedactedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Serialize for RedactedSecret {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("[REDACTED]")
    }
}

const REDACTED: &str = "[REDACTED]";

/// Query parameters and header prefixes whose value must never reach a log line or the UI.
/// Matched case-insensitively.
const SENSITIVE_MARKERS: &[&str] = &[
    "key=",
    "api_key=",
    "access_token=",
    "refresh_token=",
    "client_secret=",
    "code=",
    "code_verifier=",
    "bearer ",
];

/// Scrubs API keys, OAuth tokens, authorization codes and Bearer tokens from an error or log
/// string, plus any standalone Google `AIza...` key.
pub fn sanitize_sensitive_text(input: &str) -> String {
    let mut text = input.to_string();
    for marker in SENSITIVE_MARKERS {
        redact_values_after(&mut text, marker);
    }
    redact_google_keys(&mut text);
    text
}

/// Replaces the value that follows each (case-insensitive) occurrence of `marker`.
fn redact_values_after(text: &mut String, marker: &str) {
    let mut from = 0;
    // ASCII lowercasing keeps every byte offset the same, so positions carry over to `text`.
    while let Some(found) = text.to_ascii_lowercase()[from..].find(marker) {
        let value_start = from + found + marker.len();
        let value_end = text[value_start..]
            .find(|c: char| c.is_whitespace() || matches!(c, '&' | '"' | '\'' | ')' | ','))
            .map_or(text.len(), |offset| value_start + offset);
        if value_end > value_start {
            text.replace_range(value_start..value_end, REDACTED);
            from = value_start + REDACTED.len();
        } else {
            from = value_start;
        }
    }
}

/// Replaces anything shaped like a Google API key (`AIza` plus at least six token characters).
fn redact_google_keys(text: &mut String) {
    let mut from = 0;
    while let Some(found) = text[from..].find("AIza") {
        let start = from + found;
        let end = text[start..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .map_or(text.len(), |offset| start + offset);
        if end - start >= 10 {
            text.replace_range(start..end, REDACTED);
            from = start + REDACTED.len();
        } else {
            from = end;
        }
    }
}

/// Checks that an ID that goes into a request (channel, playlist or video) is made only of
/// `[A-Za-z0-9_.-]`, at most 128 characters, so it cannot change the path or the query string.
pub fn validate_safe_identifier<'a>(raw: &'a str, label: &str) -> Result<&'a str, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }
    if trimmed.len() > 128 {
        return Err(format!("{label} exceeds maximum length of 128 characters"));
    }
    if trimmed == "."
        || trimmed.contains("..")
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(format!(
            "{label} contains invalid characters; only alphanumeric, '_', '-', and '.' are permitted"
        ));
    }
    Ok(trimmed)
}

/// Checks that an API key has no whitespace, control characters or URL delimiters that could
/// alter the request it is sent in.
pub fn validate_api_key_format(raw: &str) -> Result<&str, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Credential value cannot be empty".into());
    }
    if trimmed.len() > 256 {
        return Err("API key exceeds maximum allowed length (256 characters)".into());
    }
    if trimmed.chars().any(|c| {
        c.is_control()
            || c.is_whitespace()
            || matches!(c, '?' | '&' | '#' | '/' | '\\' | '"' | '\'' | '<' | '>')
    }) {
        return Err(
            "API key contains invalid control, whitespace, or URL delimiter characters".into(),
        );
    }
    Ok(trimmed)
}

/// Cleans text that came from YouTube before it is shown: drops control and bidirectional override
/// characters (which can make a title lie about what it says), trims, and cuts it to `max_chars`.
pub fn sanitize_display_text(raw: &str, max_chars: usize) -> String {
    raw.chars()
        .filter(|&c| {
            !c.is_control()
                && !('\u{202A}'..='\u{202E}').contains(&c)
                && !('\u{2066}'..='\u{2069}').contains(&c)
        })
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Lets an image URL from YouTube through only if it is plain HTTPS on a YouTube image host, with
/// no credentials, custom port or path tricks. Anything else never reaches the web view.
pub fn validate_trusted_image_url(raw_url: &str) -> Option<String> {
    let trimmed = raw_url.trim();
    if trimmed.contains("..") || trimmed.contains('\\') {
        return None;
    }
    let parsed = url::Url::parse(trimmed).ok()?;
    if parsed.scheme() != "https"
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || parsed.path().contains("..")
    {
        return None;
    }
    match parsed.host_str()? {
        "yt3.ggpht.com" | "yt3.googleusercontent.com" | "i.ytimg.com" => Some(parsed.to_string()),
        _ => None,
    }
}
