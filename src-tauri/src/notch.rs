//! Pure geometry for the pill and its window. No AppKit here, so all of it is unit-tested.
//!
//! The window is a transparent rectangle that hangs from the top edge of the screen. The pill is
//! drawn inside it, centered, with a little room around it for the concave ears and the spring
//! overshoot.

use serde::{Deserialize, Serialize};

/// Expanded dashboard pill, before the camera-housing inset is added to its height.
pub const EXPANDED_PILL_WIDTH: f64 = 400.0;
pub const EXPANDED_PILL_HEIGHT: f64 = 216.0;

/// The two concave "ears" add 8 px on each side of every pill.
const EARS_WIDTH: f64 = 16.0;
/// Transparent room around the collapsed pill: the ears plus a few pixels.
const COLLAPSED_MARGIN_X: f64 = 20.0;
/// Transparent room around the dashboard, enough for the spring to overshoot (about 4%).
const EXPANDED_MARGIN_X: f64 = 40.0;
const EXPANDED_MARGIN_Y: f64 = 16.0;

/// How the pill is laid out for the screen it sits on (derived from the physical notch, if any).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutHints {
    pub peek_width: f64,
    pub peek_height: f64,
    /// Space the expanded dashboard leaves empty at the top (the camera housing, if any).
    pub expanded_inset_top: f64,
    /// Size of the invisible idle pill, which is also the hover target.
    pub idle_width: f64,
    pub idle_height: f64,
    /// True when stats are split into wings either side of a hardware notch.
    pub wings: bool,
}

impl Default for LayoutHints {
    /// Screens without a notch: a compact pill.
    fn default() -> Self {
        Self {
            peek_width: 248.0,
            peek_height: 32.0,
            expanded_inset_top: 0.0,
            idle_width: 132.0,
            idle_height: 24.0,
            wings: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotchGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub expanded: bool,
}

/// Window size while idle or peeking.
pub fn collapsed_size(hints: &LayoutHints) -> (f64, f64) {
    (
        hints.peek_width + COLLAPSED_MARGIN_X,
        hints.peek_height + 2.0,
    )
}

/// Window size while the dashboard is open.
pub fn expanded_size(hints: &LayoutHints) -> (f64, f64) {
    (
        EXPANDED_PILL_WIDTH + EXPANDED_MARGIN_X,
        EXPANDED_PILL_HEIGHT + EXPANDED_MARGIN_Y + hints.expanded_inset_top,
    )
}

/// Top-center window bounds inside a screen frame given in logical top-left coordinates.
pub fn geometry_in_frame(
    frame_x: f64,
    frame_top: f64,
    frame_width: f64,
    expanded: bool,
    hints: &LayoutHints,
) -> NotchGeometry {
    let (width, height) = if expanded {
        expanded_size(hints)
    } else {
        collapsed_size(hints)
    };
    NotchGeometry {
        x: frame_x + ((frame_width - width) / 2.0).max(0.0),
        y: frame_top,
        width,
        height,
        expanded,
    }
}

/// Which pill the user currently sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillStage {
    Idle,
    Peek,
    Expanded,
}

/// A window frame in AppKit coordinates (bottom-left origin, logical points).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Visible size of the pill for a stage, ears included.
pub fn pill_size(stage: PillStage, hints: &LayoutHints) -> (f64, f64) {
    match stage {
        PillStage::Idle => (hints.idle_width, hints.idle_height),
        PillStage::Peek => (hints.peek_width + EARS_WIDTH, hints.peek_height),
        PillStage::Expanded => (
            EXPANDED_PILL_WIDTH + EARS_WIDTH,
            EXPANDED_PILL_HEIGHT + hints.expanded_inset_top,
        ),
    }
}

/// True when the cursor is over the **visible pill** (not the transparent window around it).
/// The pill hangs from the top edge of the window, centered. A few pixels of tolerance are added
/// on the sides and bottom, and above the top so a cursor pushed against the screen edge counts.
pub fn is_cursor_in_pill(
    mouse_x: f64,
    mouse_y: f64,
    frame: &FrameRect,
    stage: PillStage,
    hints: &LayoutHints,
) -> bool {
    const PAD_SIDE: f64 = 2.0;
    const PAD_BOTTOM: f64 = 2.0;
    const PAD_TOP: f64 = 4.0;

    let all_finite = [mouse_x, mouse_y, frame.x, frame.y]
        .iter()
        .all(|value| value.is_finite());
    if !all_finite || frame.width <= 0.0 || frame.height <= 0.0 {
        return false;
    }

    let (pill_w, pill_h) = pill_size(stage, hints);
    let pill_w = pill_w.min(frame.width);
    let pill_h = pill_h.min(frame.height);
    let center_x = frame.x + frame.width / 2.0;
    let top = frame.y + frame.height;

    mouse_x >= center_x - pill_w / 2.0 - PAD_SIDE
        && mouse_x <= center_x + pill_w / 2.0 + PAD_SIDE
        && mouse_y >= top - pill_h - PAD_BOTTOM
        && mouse_y <= top + PAD_TOP
}

/// How many 25 ms ticks the cursor must rest on the idle pill before it unfolds (about 125 ms),
/// so a cursor that is only crossing the top edge of the screen does not open it.
pub const OPEN_DWELL_TICKS: u32 = 5;

/// Next hover state. Opening from idle needs a short dwell. Once the pill is open, or the
/// dashboard is showing, the cursor being inside the pill is enough.
pub fn next_hover_state(
    inside: bool,
    hovered: bool,
    expanded: bool,
    dwell_ticks: u32,
) -> (bool, u32) {
    if !inside {
        return (false, 0);
    }
    if hovered || expanded {
        return (true, dwell_ticks);
    }
    let ticks = dwell_ticks + 1;
    (ticks >= OPEN_DWELL_TICKS, ticks)
}
