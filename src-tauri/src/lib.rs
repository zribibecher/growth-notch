mod commands;
pub mod credentials;
pub mod notch;
#[cfg(target_os = "macos")]
mod panel;
pub mod screen;
pub mod security;
mod state;
pub mod telemetry;
mod tray;
#[cfg(target_os = "macos")]
mod watcher;
mod window;

use credentials::FileVault;
use screen::read_screen_metrics;
use state::AppState;
use tauri::{Emitter, Manager};

pub fn run() {
    let mut app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Launching a second copy opens the dashboard instead of doing nothing.
            let _ = app.emit("notch://open", ());
        }))
        .setup(|app| {
            // Setup runs on the main thread, which AppKit needs for the screen query.
            let metrics = read_screen_metrics().unwrap_or_default();
            let vault = match app.path().app_data_dir() {
                Ok(dir) => FileVault::open(dir.join("vault.enc")),
                Err(err) => {
                    eprintln!("growth-notch: no app data directory ({err}); credentials will not be saved");
                    FileVault::in_memory()
                }
            };
            let state = AppState::new(vault, metrics);
            let geometry = state
                .screen
                .read()
                .map(|screen| screen.geometry())
                .expect("screen state lock is not poisoned at startup");
            app.manage(state);

            let window = window::create(app, &geometry)?;
            tray::build(app)?;

            #[cfg(target_os = "macos")]
            {
                // Click-through until the cursor reaches the pill.
                window.set_ignore_cursor_events(true)?;
                panel::configure(&window)?;
                watcher::start(window.clone());
            }
            #[cfg(not(target_os = "macos"))]
            let _ = window;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::save_api_key,
            commands::delete_api_key,
            commands::get_credential_status,
            commands::refresh_telemetry,
            commands::set_notch_expanded,
            commands::set_keyboard_focus,
            commands::get_screen_info,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Growth Notch");

    // A menu bar app: no Dock icon, and it never takes focus. Set before the event loop starts,
    // because setting it later in `setup` leaves a Dock icon visible for about a second.
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    app.run(|_, _| {});
}
