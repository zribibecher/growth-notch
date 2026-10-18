//! The menu bar icon. The Dock icon is hidden, so this is how people reopen and quit the app.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Emitter;

/// 44x44 RGBA glyph with only an alpha channel. As a template image, macOS tints it to match
/// the light or dark menu bar.
const GLYPH_RGBA: &[u8] = include_bytes!("../icons/tray.rgba");
const GLYPH_SIZE: u32 = 44;

pub fn build(app: &tauri::App) -> tauri::Result<()> {
    let title = MenuItem::with_id(app, "title", "Growth Notch", false, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show Dashboard", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refresh Now", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[&title, &separator, &show, &refresh, &separator, &quit],
    )?;

    TrayIconBuilder::new()
        .icon(tauri::image::Image::new_owned(
            GLYPH_RGBA.to_vec(),
            GLYPH_SIZE,
            GLYPH_SIZE,
        ))
        .icon_as_template(true)
        .tooltip("Growth Notch")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                let _ = app.emit("notch://open", ());
            }
            "refresh" => {
                let _ = app.emit("notch://refresh", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
