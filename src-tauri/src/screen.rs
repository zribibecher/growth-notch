//! Screen geometry: where the camera notch is (if any) and how the pill should lay out around it.
//!
//! Coordinates are logical points with a **top-left** origin on the primary display, unlike AppKit's
//! bottom-left origin. `primary_height` is kept so callers can convert back for `NSEvent` math.

use crate::notch::LayoutHints;
use serde::{Deserialize, Serialize};

/// Width of the stat "wing" on each side of a hardware notch.
const WING_WIDTH: f64 = 78.0;
const MIN_PEEK_WIDTH: f64 = 248.0;
const MIN_PEEK_HEIGHT: f64 = 32.0;
/// Simulated 14-inch MacBook Pro notch, used when `NOTCH_SIM=1` (no hardware needed).
const SIM_NOTCH_WIDTH: f64 = 185.0;
const SIM_NOTCH_HEIGHT: f64 = 32.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenMetrics {
    pub has_notch: bool,
    pub notch_width: f64,
    pub notch_height: f64,
    pub menu_bar_height: f64,
    pub frame_x: f64,
    /// Top edge of the chosen screen, top-left origin on the primary display.
    pub frame_top: f64,
    pub frame_width: f64,
    pub frame_height: f64,
    pub primary_height: f64,
}

impl Default for ScreenMetrics {
    /// Fallback when AppKit can't be queried: a 1440x900 screen without a notch.
    fn default() -> Self {
        Self {
            has_notch: false,
            notch_width: 0.0,
            notch_height: 0.0,
            menu_bar_height: 24.0,
            frame_x: 0.0,
            frame_top: 0.0,
            frame_width: 1440.0,
            frame_height: 900.0,
            primary_height: 900.0,
        }
    }
}

/// Pure mapping from screen metrics to pill layout. Without a notch the compact pill is used;
/// with one, the pill is exactly as tall as the housing and stats sit in wings either side of it.
pub fn layout_for(m: &ScreenMetrics) -> LayoutHints {
    if !m.has_notch || m.notch_width <= 0.0 || m.notch_height <= 0.0 {
        return LayoutHints::default();
    }
    LayoutHints {
        peek_width: (m.notch_width + 2.0 * WING_WIDTH).max(MIN_PEEK_WIDTH),
        peek_height: m.notch_height.max(MIN_PEEK_HEIGHT),
        expanded_inset_top: m.notch_height,
        idle_width: m.notch_width,
        idle_height: m.notch_height,
        wings: true,
    }
}

/// Overlays a fake 14-inch notch on the real metrics so the notch layout can be developed and
/// screenshotted on a Mac without one.
pub fn with_simulated_notch(mut m: ScreenMetrics) -> ScreenMetrics {
    m.has_notch = true;
    m.notch_width = SIM_NOTCH_WIDTH;
    m.notch_height = SIM_NOTCH_HEIGHT;
    m
}

pub fn simulation_requested() -> bool {
    std::env::var("NOTCH_SIM")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Reads the metrics of the screen the pill should live on: the built-in notched display if there
/// is one, otherwise the primary display. Must run on the main thread.
#[cfg(target_os = "macos")]
pub fn read_screen_metrics() -> Option<ScreenMetrics> {
    use objc2_app_kit::NSScreen;
    use objc2_foundation::MainThreadMarker;

    let mtm = MainThreadMarker::new()?;
    let screens = NSScreen::screens(mtm);
    let primary = screens.iter().next()?;
    let primary_height = primary.frame().size.height;

    let chosen = screens
        .iter()
        .find(|s| s.safeAreaInsets().top > 0.0)
        .unwrap_or_else(|| primary.clone());

    let frame = chosen.frame();
    let visible = chosen.visibleFrame();
    let insets = chosen.safeAreaInsets();
    let left = chosen.auxiliaryTopLeftArea();
    let right = chosen.auxiliaryTopRightArea();

    let has_notch = insets.top > 0.0 && left.size.width > 0.0 && right.size.width > 0.0;
    let notch_width = if has_notch {
        (frame.size.width - left.size.width - right.size.width).max(0.0)
    } else {
        0.0
    };

    let metrics = ScreenMetrics {
        has_notch,
        notch_width,
        notch_height: if has_notch { insets.top } else { 0.0 },
        menu_bar_height: (frame.origin.y + frame.size.height)
            - (visible.origin.y + visible.size.height),
        frame_x: frame.origin.x,
        frame_top: primary_height - (frame.origin.y + frame.size.height),
        frame_width: frame.size.width,
        frame_height: frame.size.height,
        primary_height,
    };

    Some(if simulation_requested() {
        with_simulated_notch(metrics)
    } else {
        metrics
    })
}

#[cfg(not(target_os = "macos"))]
pub fn read_screen_metrics() -> Option<ScreenMetrics> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_notch_uses_compact_pill() {
        let hints = layout_for(&ScreenMetrics::default());
        assert_eq!(hints, LayoutHints::default());
        assert!(!hints.wings);
        assert_eq!(hints.expanded_inset_top, 0.0);
    }

    #[test]
    fn notch_pill_matches_housing_and_splits_into_wings() {
        let m = with_simulated_notch(ScreenMetrics::default());
        let hints = layout_for(&m);
        assert!(hints.wings);
        assert_eq!(hints.peek_height, 32.0);
        assert_eq!(hints.expanded_inset_top, 32.0);
        assert_eq!(hints.peek_width, 185.0 + 2.0 * WING_WIDTH);
        assert_eq!(hints.idle_width, 185.0);
        assert_eq!(hints.idle_height, 32.0);
    }

    #[test]
    fn degenerate_notch_values_fall_back_to_default() {
        // flag set but no size
        let m = ScreenMetrics {
            has_notch: true,
            ..ScreenMetrics::default()
        };
        assert_eq!(layout_for(&m), LayoutHints::default());
    }
}
