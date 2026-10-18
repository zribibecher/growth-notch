//! The commands the web view may call. Each one is also listed in `build.rs` and in
//! `capabilities/default.json`, so nothing else is reachable from the page.

use crate::credentials::CredentialStatus;
use crate::notch::{LayoutHints, NotchGeometry};
use crate::screen::ScreenMetrics;
use crate::security::CredentialProvider;
use crate::state::AppState;
use crate::telemetry::TelemetrySnapshot;
use crate::window;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn save_api_key(
    state: State<'_, AppState>,
    provider: String,
    api_key: String,
) -> Result<CredentialStatus, String> {
    let provider = CredentialProvider::from_key(&provider)?;
    state.vault.set_secret(provider, &api_key)?;
    state.vault.status()
}

#[tauri::command]
pub async fn delete_api_key(
    state: State<'_, AppState>,
    provider: String,
) -> Result<CredentialStatus, String> {
    let provider = CredentialProvider::from_key(&provider)?;
    state.vault.delete_secret(provider)?;
    state.vault.status()
}

#[tauri::command]
pub async fn get_credential_status(state: State<'_, AppState>) -> Result<CredentialStatus, String> {
    state.vault.status()
}

#[tauri::command]
pub async fn refresh_telemetry(state: State<'_, AppState>) -> Result<TelemetrySnapshot, String> {
    Ok(state.telemetry.poll_all(&state.vault).await)
}

/// Screen metrics plus the pill layout derived from them, for the React UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenInfo {
    pub metrics: ScreenMetrics,
    pub layout: LayoutHints,
}

#[tauri::command]
pub fn get_screen_info(state: State<'_, AppState>) -> Result<ScreenInfo, String> {
    let screen = state
        .screen
        .read()
        .map_err(|_| "screen state is unavailable".to_string())?;
    Ok(ScreenInfo {
        metrics: screen.metrics,
        layout: screen.hints,
    })
}

/// Resizes the native window to match the dashboard (or the collapsed pill).
#[tauri::command]
pub async fn set_notch_expanded(
    app: AppHandle,
    state: State<'_, AppState>,
    expanded: bool,
) -> Result<NotchGeometry, String> {
    let window = app
        .get_webview_window(window::WINDOW_LABEL)
        .ok_or_else(|| "Notch window not found".to_string())?;

    let geometry = {
        let mut screen = state
            .screen
            .write()
            .map_err(|_| "screen state is unavailable".to_string())?;
        screen.expanded = expanded;
        screen.geometry()
    };
    window::apply_geometry(&window, &geometry).map_err(|e| e.to_string())?;
    Ok(geometry)
}

/// Lets the Setup field take keyboard focus, and gives it back to your app everywhere else.
#[tauri::command]
pub fn set_keyboard_focus(app: AppHandle, allow: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if let Some(window) = app.get_webview_window(window::WINDOW_LABEL) {
        crate::panel::set_key_allowed(&window, allow);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, allow);
    Ok(())
}
