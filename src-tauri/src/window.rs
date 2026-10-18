//! The notch window: creating it, moving it, and keeping it on the right screen.

use crate::notch::NotchGeometry;
use crate::screen::{layout_for, read_screen_metrics};
use crate::state::AppState;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewWindow, WebviewWindowBuilder,
};

pub const WINDOW_LABEL: &str = "notch";

/// Builds the window from the `notch` entry in `tauri.conf.json` and places it at `geometry`.
pub fn create(app: &tauri::App, geometry: &NotchGeometry) -> tauri::Result<WebviewWindow> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == WINDOW_LABEL)
        .ok_or(tauri::Error::WindowNotFound)?;
    let window = WebviewWindowBuilder::from_config(app, config)?.build()?;
    apply_geometry(&window, geometry)?;
    Ok(window)
}

pub fn apply_geometry(window: &WebviewWindow, geometry: &NotchGeometry) -> tauri::Result<()> {
    window.set_size(LogicalSize::new(geometry.width, geometry.height))?;
    window.set_position(LogicalPosition::new(geometry.x, geometry.y))
}

/// Re-reads the screen layout. If it changed (display hot-plug, resolution, a notch appearing),
/// updates the shared state, moves the window and tells the UI. Must run on the main thread.
pub fn refresh_screen(app: &AppHandle) {
    let Some(metrics) = read_screen_metrics() else {
        return;
    };
    let state = app.state::<AppState>();
    let (geometry, payload) = {
        let Ok(mut screen) = state.screen.write() else {
            return;
        };
        if screen.metrics == metrics {
            return;
        }
        screen.metrics = metrics;
        screen.hints = layout_for(&metrics);
        (
            screen.geometry(),
            serde_json::json!({ "metrics": screen.metrics, "layout": screen.hints }),
        )
    };
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        let _ = apply_geometry(&window, &geometry);
    }
    let _ = app.emit("notch://layout", payload);
}
