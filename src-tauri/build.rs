fn main() {
    // Only these commands are reachable from the web view. Each one is then granted explicitly in
    // `capabilities/default.json`; anything not listed here cannot be invoked at all.
    let commands = &[
        "save_api_key",
        "delete_api_key",
        "get_credential_status",
        "refresh_telemetry",
        "set_notch_expanded",
        "set_keyboard_focus",
        "get_screen_info",
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(commands)),
    )
    .expect("failed to run the Tauri build script");
}
