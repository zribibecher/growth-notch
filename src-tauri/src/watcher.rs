//! Decides when the pill unfolds, by polling the global cursor position.
//!
//! WebKit does not deliver hover events to a window that is not focused, so the cursor
//! (`NSEvent.mouseLocation`) is compared with the **visible pill** instead. While the cursor is
//! not over the pill the window ignores the mouse, so its transparent margins never block clicks
//! or scrolling in the page underneath.

use crate::notch::{is_cursor_in_pill, next_hover_state, FrameRect, PillStage};
use crate::panel::haptic_tick;
use crate::state::AppState;
use crate::window::refresh_screen;
use objc2_app_kit::NSEvent;
use std::time::Duration;
use tauri::{Emitter, Manager, WebviewWindow};

/// Poll quickly near the pill, and slowly when the cursor is far away.
const FAST_MS: u64 = 25;
const SLOW_MS: u64 = 100;
/// A cursor this close to the window (points) switches polling to the fast rate.
const NEAR: f64 = 80.0;
/// How often to check whether the screen layout changed.
const SCREEN_CHECK_MS: u64 = 1500;

/// Starts the watcher. Set `GROWTH_NOTCH_DEBUG=1` to print every hover change to the terminal.
pub fn start(window: WebviewWindow) {
    let app = window.app_handle().clone();
    let screen = app.state::<AppState>().screen.clone();
    let debug = std::env::var_os("GROWTH_NOTCH_DEBUG").is_some();

    tauri::async_runtime::spawn(async move {
        let mut hovered = false;
        let mut dwell_ticks: u32 = 0;
        let mut since_screen_check_ms: u64 = 0;

        loop {
            // Copy what is needed out of the lock so no guard is held across an await.
            let snapshot = screen
                .read()
                .ok()
                .map(|s| (s.geometry(), s.hints, s.metrics.primary_height, s.expanded));
            let Some((geometry, hints, primary_height, expanded)) = snapshot else {
                tokio::time::sleep(Duration::from_millis(SLOW_MS)).await;
                continue;
            };

            let mouse = NSEvent::mouseLocation();
            // The geometry is top-left based; NSEvent is bottom-left based.
            let frame = FrameRect {
                x: geometry.x,
                y: primary_height - (geometry.y + geometry.height),
                width: geometry.width,
                height: geometry.height,
            };
            let stage = if expanded {
                PillStage::Expanded
            } else if hovered {
                PillStage::Peek
            } else {
                PillStage::Idle
            };
            let inside = is_cursor_in_pill(mouse.x, mouse.y, &frame, stage, &hints);
            let (next, ticks) = next_hover_state(inside, hovered, expanded, dwell_ticks);
            dwell_ticks = ticks;

            if next != hovered {
                if debug {
                    eprintln!(
                        "growth-notch: hover {hovered} -> {next} stage={stage:?} mouse=({:.0},{:.0}) frame=({:.0},{:.0},{:.0},{:.0})",
                        mouse.x, mouse.y, frame.x, frame.y, frame.width, frame.height
                    );
                }
                hovered = next;
                let _ = window.set_ignore_cursor_events(!hovered);
                if hovered {
                    let _ = app.run_on_main_thread(haptic_tick);
                }
                let _ = window.emit("notch://hover", hovered);
            }

            let near = mouse.x >= frame.x - NEAR
                && mouse.x <= frame.x + frame.width + NEAR
                && mouse.y >= frame.y - NEAR
                && mouse.y <= frame.y + frame.height + NEAR;
            let sleep_ms = if near || hovered { FAST_MS } else { SLOW_MS };
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;

            since_screen_check_ms += sleep_ms;
            if since_screen_check_ms >= SCREEN_CHECK_MS {
                since_screen_check_ms = 0;
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || refresh_screen(&handle));
            }
        }
    });
}
