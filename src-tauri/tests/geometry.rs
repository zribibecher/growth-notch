use growth_notch_lib::notch::{
    collapsed_size, expanded_size, geometry_in_frame, is_cursor_in_pill, next_hover_state,
    FrameRect, LayoutHints, PillStage, OPEN_DWELL_TICKS,
};
use growth_notch_lib::screen::{layout_for, with_simulated_notch, ScreenMetrics};

/// A window frame hanging from the top of a screen, in AppKit coordinates (origin bottom-left).
fn top_frame(center_x: f64, width: f64, height: f64, screen_top: f64) -> FrameRect {
    FrameRect {
        x: center_x - width / 2.0,
        y: screen_top - height,
        width,
        height,
    }
}

const TOP: f64 = 900.0;

#[test]
fn without_a_notch_the_windows_keep_their_compact_sizes() {
    let plain = LayoutHints::default();
    assert_eq!(collapsed_size(&plain), (268.0, 34.0));
    assert_eq!(expanded_size(&plain), (440.0, 232.0));

    let window = geometry_in_frame(0.0, 0.0, 1440.0, false, &plain);
    assert_eq!(window.x, (1440.0 - 268.0) / 2.0);
    assert_eq!(window.y, 0.0);
    assert!(!window.expanded);
}

#[test]
fn on_a_notched_screen_the_dashboard_starts_below_the_camera() {
    let notched = layout_for(&with_simulated_notch(ScreenMetrics::default()));
    // A screen that does not start at x = 0, as with a second display.
    let window = geometry_in_frame(1440.0, 0.0, 1512.0, true, &notched);

    assert_eq!(window.width, 440.0);
    assert_eq!(window.height, 232.0 + 32.0);
    assert_eq!(window.x, 1440.0 + (1512.0 - 440.0) / 2.0);
    assert!(window.expanded);
}

#[test]
fn a_window_wider_than_the_screen_stays_on_it() {
    let window = geometry_in_frame(100.0, 0.0, 300.0, true, &LayoutHints::default());
    assert_eq!(window.x, 100.0);
}

#[test]
fn only_the_visible_pill_counts_as_hovered() {
    let hints = LayoutHints::default(); // idle 132 x 24, peek 248 x 32 (+16 for the ears)
    let frame = top_frame(720.0, 268.0, 34.0, TOP);
    let inside = |x: f64, y: f64, stage| is_cursor_in_pill(x, y, &frame, stage, &hints);

    // The idle pill is smaller than the 268 x 34 window around it.
    assert!(inside(720.0, TOP - 10.0, PillStage::Idle));
    assert!(inside(784.0, TOP - 10.0, PillStage::Idle));
    assert!(!inside(800.0, TOP - 10.0, PillStage::Idle));
    assert!(!inside(720.0, TOP - 30.0, PillStage::Idle));

    // A cursor pushed against the top edge of the screen still counts.
    assert!(inside(720.0, TOP + 2.0, PillStage::Idle));
    assert!(!inside(720.0, TOP + 8.0, PillStage::Idle));

    // The peek pill is wider.
    assert!(inside(840.0, TOP - 10.0, PillStage::Peek));
    assert!(!inside(857.0, TOP - 10.0, PillStage::Peek));
}

#[test]
fn the_open_dashboard_ignores_its_transparent_margins() {
    let hints = LayoutHints::default();
    let (width, height) = expanded_size(&hints); // 440 x 232 around a 416 x 216 pill
    let frame = top_frame(720.0, width, height, TOP);
    let inside = |x: f64, y: f64| is_cursor_in_pill(x, y, &frame, PillStage::Expanded, &hints);

    assert!(inside(720.0, TOP - 100.0));
    assert!(!inside(937.0, TOP - 100.0), "right margin");
    assert!(!inside(720.0, TOP - 225.0), "bottom margin");
}

#[test]
fn nonsense_coordinates_are_never_a_hover() {
    let hints = LayoutHints::default();
    let frame = top_frame(720.0, 268.0, 34.0, TOP);
    for (x, y) in [
        (f64::NAN, 890.0),
        (720.0, f64::INFINITY),
        (f64::NEG_INFINITY, f64::NAN),
    ] {
        assert!(!is_cursor_in_pill(x, y, &frame, PillStage::Idle, &hints));
    }
    let empty = FrameRect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };
    assert!(!is_cursor_in_pill(
        0.0,
        0.0,
        &empty,
        PillStage::Idle,
        &hints
    ));
}

#[test]
fn opening_needs_a_short_dwell_but_staying_open_does_not() {
    let dwell = |ticks: u32| {
        (0..ticks).fold((false, 0), |(_, so_far), _| {
            next_hover_state(true, false, false, so_far)
        })
    };

    // A cursor that only crosses the top edge never opens the pill, and leaving resets the count.
    let (open, ticks) = dwell(OPEN_DWELL_TICKS - 1);
    assert!(!open);
    assert_eq!(next_hover_state(false, false, false, ticks), (false, 0));

    // Resting on it for the whole dwell opens it.
    assert!(dwell(OPEN_DWELL_TICKS).0);

    // Once it is open, or the dashboard is up, being inside is enough; leaving closes at once.
    assert!(next_hover_state(true, true, false, 0).0);
    assert!(next_hover_state(true, false, true, 0).0);
    assert_eq!(next_hover_state(false, true, false, 0), (false, 0));
}
