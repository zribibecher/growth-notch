//! AppKit tweaks that make the window behave like part of the screen: a non-activating panel at
//! status-bar level that joins every Space, plus the haptic tick. macOS only.

use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, NSObjectProtocol, Sel};
use objc2::{sel, ClassType};
use objc2_app_kit::{
    NSColor, NSHapticFeedbackManager, NSHapticFeedbackPattern, NSHapticFeedbackPerformanceTime,
    NSHapticFeedbackPerformer, NSPanel, NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior,
    NSWindowStyleMask,
};
use std::ffi::{c_void, CStr};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::WebviewWindow;

const PANEL_CLASS_NAME: &CStr = c"GrowthNotchPanel";

/// Whether the panel may become the key window. Keyboard focus is only needed while the Setup
/// field is open; everywhere else the panel must never take typing away from the app you are using.
static MAY_BE_KEY: AtomicBool = AtomicBool::new(false);

extern "C" fn can_become_key(_this: &AnyObject, _cmd: Sel) -> Bool {
    Bool::new(MAY_BE_KEY.load(Ordering::SeqCst))
}

extern "C" fn can_become_main(_this: &AnyObject, _cmd: Sel) -> Bool {
    Bool::NO
}

/// # Safety
/// `ptr` must be the `NSWindow` of a live window, and the caller must be on the main thread.
unsafe fn ns_window<'a>(ptr: *mut c_void) -> Option<&'a NSWindow> {
    unsafe { ptr.cast::<NSWindow>().as_ref() }
}

/// Allows or forbids keyboard focus for the panel. Forbidding it also resigns key status, which
/// hands typing back to the app you were using.
pub fn set_key_allowed(window: &WebviewWindow, allow: bool) {
    MAY_BE_KEY.store(allow, Ordering::SeqCst);
    let _ = window.with_webview(move |webview| {
        // SAFETY: Tauri runs this closure on the main thread, with the window's own NSWindow.
        let Some(ns_window) = (unsafe { ns_window(webview.ns_window()) }) else {
            return;
        };
        if allow {
            ns_window.makeKeyWindow();
        } else {
            ns_window.resignKeyWindow();
        }
    });
}

/// Light "alignment" tick on Force Touch trackpads (no-op on mice and when no finger is on the pad).
pub fn haptic_tick() {
    NSHapticFeedbackManager::defaultPerformer().performFeedbackPattern_performanceTime(
        NSHapticFeedbackPattern::Alignment,
        NSHapticFeedbackPerformanceTime::Now,
    );
}

/// Turns the window into a transparent, shadowless panel at the status-bar level that joins every
/// Space and can sit over full-screen apps. Must run on the main thread.
pub fn configure(window: &WebviewWindow) -> tauri::Result<()> {
    // SAFETY: called from `setup`, on the main thread, with the window Tauri just created.
    let Some(ns_window) = (unsafe { ns_window(window.ns_window()?) }) else {
        return Ok(());
    };

    if promote_to_nonactivating_panel(ns_window) {
        ns_window.setStyleMask(ns_window.styleMask() | NSWindowStyleMask::NonactivatingPanel);
        ns_window.setHidesOnDeactivate(false);
    } else {
        eprintln!("growth-notch: could not turn the window into a panel; full-screen Spaces are unsupported");
    }

    ns_window.setLevel(NSStatusWindowLevel);
    ns_window.setOpaque(false);
    ns_window.setHasShadow(false);
    ns_window.setAcceptsMouseMovedEvents(true);
    ns_window.setBackgroundColor(Some(&NSColor::clearColor()));
    ns_window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    Ok(())
}

/// Re-classes the Tauri-created window as an `NSPanel` subclass. macOS only draws non-activating
/// panels over other apps' full-screen Spaces, and a panel can take typing (the Setup field)
/// without activating the app. Returns `false`, leaving the window untouched, if the swap is not
/// safe or can't be verified; the widget then still works, but won't draw over full-screen apps.
fn promote_to_nonactivating_panel(ns_window: &NSWindow) -> bool {
    let panel_class = match AnyClass::get(PANEL_CLASS_NAME) {
        Some(existing) => existing,
        None => {
            let Some(mut builder) = ClassBuilder::new(PANEL_CLASS_NAME, NSPanel::class()) else {
                return false;
            };
            // SAFETY: both overrides take `self` and the selector and return a `BOOL`, exactly
            // like the `NSWindow` methods they replace.
            unsafe {
                builder.add_method(
                    sel!(canBecomeKeyWindow),
                    can_become_key as extern "C" fn(_, _) -> _,
                );
                builder.add_method(
                    sel!(canBecomeMainWindow),
                    can_become_main as extern "C" fn(_, _) -> _,
                );
            }
            builder.register()
        }
    };

    // The existing object must be at least as large as the new class expects.
    if panel_class.instance_size() > AnyObject::class(ns_window).instance_size() {
        return false;
    }
    // SAFETY: `NSPanel` descends from `NSWindow`, the size was checked above, and the overridden
    // methods are compatible with the originals.
    unsafe { AnyObject::set_class(ns_window, panel_class) };
    ns_window.isKindOfClass(NSPanel::class())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panel_is_no_bigger_than_a_window() {
        // tao's window class is an NSWindow plus one flag, so the swap is only safe while NSPanel
        // adds no instance variables of its own. This catches a future macOS that changes that.
        assert!(NSPanel::class().instance_size() <= NSWindow::class().instance_size());
    }
}
