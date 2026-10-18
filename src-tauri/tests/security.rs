use growth_notch_lib::security::{
    sanitize_display_text, sanitize_sensitive_text, validate_api_key_format,
    validate_safe_identifier, validate_trusted_image_url, CredentialProvider, RedactedSecret,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(crate_dir().join(relative))
        .unwrap_or_else(|e| panic!("{relative}: {e}"))
}

fn read_json(relative: &str) -> serde_json::Value {
    serde_json::from_str(&read(relative)).unwrap()
}

/// Every `.ts` and `.tsx` file of the web UI, tests excluded.
fn web_sources() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "__tests__") {
                    walk(&path, out);
                }
            } else if path
                .extension()
                .is_some_and(|ext| ext == "ts" || ext == "tsx")
            {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(&crate_dir().join("../src"), &mut files);
    files
}

// ---------------------------------------------------------------- secrets and scrubbing

#[test]
fn a_secret_never_prints_or_serializes() {
    let raw = "AIzaSySuperSecretYouTubeKey_987654321";
    let secret = RedactedSecret::new(raw);

    for shown in [
        format!("{secret:?}"),
        format!("{secret}"),
        serde_json::to_string(&secret).unwrap(),
    ] {
        assert!(!shown.contains(raw), "{shown}");
    }
    assert_eq!(secret.to_string(), "[REDACTED]");
    assert_eq!(secret.expose_secret(), raw);
}

#[test]
fn the_scrubber_removes_keys_tokens_and_codes() {
    let error = "Request to https://www.googleapis.com/youtube/v3/channels?part=statistics&id=UC123&key=AIzaSySuperSecretYouTubeKey_987654321 failed with Authorization: Bearer fake_test_token_555 and code=oauth_temp_code_111";
    let scrubbed = sanitize_sensitive_text(error);

    for secret in [
        "AIzaSySuperSecretYouTubeKey_987654321",
        "fake_test_token_555",
        "oauth_temp_code_111",
    ] {
        assert!(!scrubbed.contains(secret), "{scrubbed}");
    }
    assert!(scrubbed.contains("key=[REDACTED]"));
    assert!(scrubbed.contains("Bearer [REDACTED]"));
    assert!(scrubbed.contains("code=[REDACTED]"));
    // Only the secrets changed.
    assert!(scrubbed.contains("part=statistics&id=UC123&"));
}

#[test]
fn the_scrubber_ignores_letter_case_and_finds_loose_keys() {
    let scrubbed = sanitize_sensitive_text(
        "key AIzaSyB1234567890abcdef rejected; KEY=secret123&ACCESS_TOKEN=tok456 and BEARER upper_tok_789",
    );
    for secret in [
        "AIzaSyB1234567890abcdef",
        "secret123",
        "tok456",
        "upper_tok_789",
    ] {
        assert!(!scrubbed.contains(secret), "{scrubbed}");
    }
}

#[test]
fn the_scrubber_handles_repeats_and_leaves_ordinary_text_alone() {
    let twice = sanitize_sensitive_text("key=aaa1 then key=bbb2");
    assert_eq!(twice, "key=[REDACTED] then key=[REDACTED]");

    for harmless in [
        "The daily YouTube quota is used up.",
        "ends with an empty value key=",
        "AIzaX is too short to be a key",
        "",
    ] {
        assert_eq!(sanitize_sensitive_text(harmless), harmless);
    }
}

#[test]
fn only_known_credential_names_are_accepted() {
    assert_eq!(
        CredentialProvider::from_key("youtube").unwrap(),
        CredentialProvider::YoutubeApiKey
    );
    assert_eq!(
        CredentialProvider::from_key(" YOUTUBE_CHANNEL_ID ").unwrap(),
        CredentialProvider::YoutubeChannelId
    );
    let error = CredentialProvider::from_key("../../etc/passwd?key=AIzaSyShouldBeScrubbed_123")
        .unwrap_err();
    assert!(!error.contains("AIzaSyShouldBeScrubbed_123"), "{error}");
}

// ---------------------------------------------------------------- validators

#[test]
fn ids_cannot_change_the_path_or_query_of_a_request() {
    for good in ["UC_TECH_UNTANGLED-01", "UCabc.def", "vid_1"] {
        assert!(validate_safe_identifier(good, "ID").is_ok(), "{good}");
    }
    for bad in [
        "",
        " ",
        "../../v1/admin",
        "UC123?alt=media&key=leak",
        "a/b",
        "a b",
        "..",
        &"x".repeat(129),
    ] {
        assert!(validate_safe_identifier(bad, "ID").is_err(), "{bad:?}");
    }
}

#[test]
fn api_keys_cannot_change_the_request_they_travel_in() {
    assert!(validate_api_key_format("AIzaSyValidKey_123-abc").is_ok());
    for bad in [
        "   ",
        "AIzaSyKey&extra=1",
        "AIzaSyKey?foo=bar",
        "AIzaSyKey#fragment",
        "AIzaSyKey/../../path",
        "AIzaSyKey\r\nHeader: injected",
        "AIzaSyKey with spaces",
        &"A".repeat(257),
    ] {
        assert!(validate_api_key_format(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn only_https_youtube_image_hosts_get_through() {
    for good in [
        "https://yt3.ggpht.com/ytc/AIdro_k=s176",
        "https://yt3.googleusercontent.com/ytc/avatar=s800",
        "https://i.ytimg.com/vi/abc/hqdefault.jpg",
    ] {
        assert!(validate_trusted_image_url(good).is_some(), "{good}");
    }
    for bad in [
        "http://yt3.ggpht.com/ytc/AIdro_k",
        "https://user:pass@yt3.ggpht.com/ytc/a",
        "https://yt3.ggpht.com:8443/ytc/a",
        "https://yt3.ggpht.com/ytc/../../etc",
        "https://yt3.ggpht.com.evil.com/a",
        "https://evil.example.com/pixel.png",
        "javascript:alert(1)",
        "not a url",
    ] {
        assert!(validate_trusted_image_url(bad).is_none(), "{bad}");
    }
}

#[test]
fn display_text_loses_control_and_direction_override_characters() {
    let clean = sanitize_display_text("Tech\u{202E}gnp.exe\u{202C}\0\n\r Channel", 100);
    assert_eq!(clean, "Techgnp.exe Channel");
    assert_eq!(sanitize_display_text(&"X".repeat(200), 40).len(), 40);
    assert_eq!(sanitize_display_text("  padded  ", 40), "padded");
}

// ---------------------------------------------------------------- what the web view may do

/// The names of the commands the web view can call must be the same in three places:
/// the build script (which creates the permissions), the handler list, and the capability.
#[test]
fn the_web_view_can_only_call_the_commands_listed_in_one_place() {
    let quoted = |text: &str| -> BTreeSet<String> {
        text.split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect()
    };

    let build_rs = read("build.rs");
    let from_build = quoted(between(&build_rs, "let commands = &[", "];"));

    let lib_rs = read("src/lib.rs");
    let from_handler: BTreeSet<String> = between(&lib_rs, "generate_handler![", "]")
        .split(',')
        .filter_map(|item| item.trim().strip_prefix("commands::"))
        .map(str::to_string)
        .collect();

    let capability = read_json("capabilities/default.json");
    let from_capability: BTreeSet<String> = capability["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p.as_str()?.strip_prefix("allow-"))
        .map(|name| name.replace('-', "_"))
        .collect();

    assert!(!from_build.is_empty());
    assert_eq!(
        from_build, from_handler,
        "build.rs and generate_handler! differ"
    );
    assert_eq!(
        from_build, from_capability,
        "build.rs and the capability differ"
    );
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let from = text
        .find(start)
        .unwrap_or_else(|| panic!("{start} not found"))
        + start.len();
    let to = text[from..].find(end).unwrap() + from;
    &text[from..to]
}

#[test]
fn the_capability_grants_events_and_our_own_commands_only() {
    let capability = read_json("capabilities/default.json");
    assert_eq!(capability["windows"], serde_json::json!(["notch"]));
    for permission in capability["permissions"].as_array().unwrap() {
        let permission = permission.as_str().unwrap();
        assert!(
            permission.starts_with("allow-") || permission.starts_with("core:event:"),
            "unexpected permission {permission}"
        );
    }
}

#[test]
fn no_plugins_beyond_single_instance_are_installed() {
    let cargo = read("Cargo.toml");
    let plugins: Vec<&str> = cargo
        .lines()
        .filter(|line| line.starts_with("tauri-plugin-"))
        .map(|line| line.split(' ').next().unwrap())
        .collect();
    assert_eq!(plugins, ["tauri-plugin-single-instance"]);

    let conf = read_json("tauri.conf.json");
    assert!(conf.get("plugins").is_none(), "plugin config found");
    let text = conf.to_string();
    assert!(
        !text.contains("deep-link") && !text.contains("shell"),
        "{text}"
    );
}

#[test]
fn the_content_security_policy_only_allows_the_app_itself() {
    let conf = read_json("tauri.conf.json");
    let csp = conf["app"]["security"]["csp"].as_str().unwrap();
    for directive in [
        "default-src 'self'",
        "script-src 'self'",
        "object-src 'none'",
        "base-uri 'self'",
        "form-action 'none'",
        "frame-ancestors 'none'",
        "style-src 'self'",
        "connect-src 'self' ipc: http://ipc.localhost",
        "img-src 'self' data: https://yt3.ggpht.com https://yt3.googleusercontent.com https://i.ytimg.com",
    ] {
        assert!(csp.contains(directive), "missing `{directive}` in {csp}");
    }
    assert!(!csp.contains("unsafe-"), "{csp}");
    assert_eq!(conf["app"]["withGlobalTauri"], false);
}

#[test]
fn the_window_is_see_through_and_never_takes_focus() {
    let conf = read_json("tauri.conf.json");
    let window = &conf["app"]["windows"][0];
    assert_eq!(window["label"], "notch");
    assert_eq!(window["create"], false, "the window is built in code");
    for (flag, expected) in [
        ("transparent", true),
        ("decorations", false),
        ("alwaysOnTop", true),
        ("focus", false),
        ("acceptFirstMouse", true),
        ("shadow", false),
        ("skipTaskbar", true),
        ("resizable", false),
    ] {
        assert_eq!(window[flag], expected, "{flag}");
    }
    assert_eq!(conf["app"]["macOSPrivateApi"], true);
}

#[test]
fn sources_hold_no_real_keys_and_write_nothing_to_browser_storage() {
    let mut sources = vec![
        crate_dir().join("build.rs"),
        crate_dir().join("../index.html"),
    ];
    sources.extend(
        std::fs::read_dir(crate_dir().join("src"))
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs")),
    );
    sources.extend(web_sources());

    for file in sources {
        let text = std::fs::read_to_string(&file).unwrap();
        for forbidden in [
            "AIzaSy",
            "localStorage.setItem",
            "sessionStorage.setItem",
            "indexedDB",
        ] {
            assert!(!text.contains(forbidden), "`{forbidden}` found in {file:?}");
        }
    }
    assert!(!crate_dir().join("../.env").exists());
    assert!(!crate_dir().join("../credentials.json").exists());
}
