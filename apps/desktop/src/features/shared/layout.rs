//! Fixed layout dimensions of the window regions.

use gpui::{px, Pixels};

pub const SIDEBAR_W: Pixels = px(240.);
/// How far the sidebar slides in when shown again (Cmd+B).
pub const SIDEBAR_SLIDE: Pixels = px(24.);
/// Sidebar collapsed to an icon rail, used while the detail pane is open in a narrow window.
pub const SIDEBAR_RAIL_W: Pixels = px(60.);
/// Max width of the centered list content column.
pub const CONTENT_MAX_W: Pixels = px(1360.);
/// Triage preview beside the full-width list.
pub const PREVIEW_W: Pixels = px(360.);
/// Preview description excerpt: about six lines of body text, then a fade.
pub const PREVIEW_EXCERPT_H: Pixels = px(132.);
/// Below this window width an open preview collapses the sidebar to the rail.
pub const PREVIEW_COMPACT_BELOW: Pixels = px(1100.);
/// The description editor is at least this tall, or as tall as the rendered description it replaces.
pub const DESC_EDITOR_MIN_H: Pixels = px(240.);
/// Review mode side panes: file tree and right panel bounds.
pub const TREE_MIN_W: Pixels = px(180.);
pub const TREE_MAX_W: Pixels = px(480.);
pub const RIGHT_MIN_W: Pixels = px(300.);
/// The diff never gets narrower than this when the right panel grows.
pub const DIFF_MIN_W: Pixels = px(420.);
/// Default right panel share of the content width (window minus file tree): diff 2/3, panel 1/3.
pub const RIGHT_RATIO: f32 = 1. / 3.;
/// Bounds of a dragged right panel share.
pub const RIGHT_RATIO_MIN: f32 = 0.15;
pub const RIGHT_RATIO_MAX: f32 = 0.6;

/// Right panel width for a content width and share, within its min width and the diff's min width.
pub fn right_pane_w(content: Pixels, ratio: f32) -> Pixels {
    let max = (content - DIFF_MIN_W).max(RIGHT_MIN_W);
    (content * ratio).clamp(RIGHT_MIN_W, max)
}

/// Share to remember after dragging the right panel to `right` pixels of `content`.
pub fn right_ratio(content: Pixels, right: Pixels) -> f32 {
    if content <= px(0.) {
        return RIGHT_RATIO;
    }
    (right / content).clamp(RIGHT_RATIO_MIN, RIGHT_RATIO_MAX)
}
/// Room for the traffic lights when the sidebar is hidden.
pub const TRAFFIC_LIGHTS_W: Pixels = px(84.);
/// Hit area of a pane's resize handle.
pub const RESIZE_HANDLE_W: Pixels = px(6.);
/// Margin around the floating sidebar (light theme).
pub const SIDEBAR_INSET: Pixels = px(8.);
/// Top padding of the sidebar, clearing the traffic lights.
pub const SIDEBAR_TOP: Pixels = px(52.);
/// Top padding of the list and detail columns, under the transparent titlebar.
pub const TITLEBAR_INSET: Pixels = px(48.);
/// Review mode top bar: title line plus a muted meta line.
pub const REVIEW_BAR_H: Pixels = px(54.);
/// Scroll past end: the diff can scroll its last row up to this fraction of the viewport from the top.
pub const SCROLL_PAST_END: f32 = 0.4;
/// Scroll past end spacer before the diff list has been measured.
pub const SCROLL_PAST_END_FALLBACK: Pixels = px(320.);
/// Comfortable end padding of the PR list, the timeline and the file tree.
pub const END_PAD: Pixels = px(96.);
pub const TIME_COL_W: Pixels = px(72.);
/// Settings dialog size and its left navigation.
pub const SETTINGS_W: Pixels = px(780.);
pub const SETTINGS_H: Pixels = px(560.);
pub const SETTINGS_NAV_W: Pixels = px(188.);
/// Code tab: one diff line, the file card header, the `@@` band and line number gutters.
pub const DIFF_LINE_H: Pixels = px(20.);
pub const DIFF_HEADER_H: Pixels = px(40.);
pub const DIFF_HUNK_H: Pixels = px(28.);
pub const DIFF_GUTTER_W: Pixels = px(40.);
/// Approximate mono glyph width as a fraction of the font size, for horizontal scroll limits.
pub const MONO_CHAR_RATIO: f32 = 0.62;
/// Narrowest width of the add field after the chips in the Filters chip editors.
pub const CHIP_INPUT_MIN_W: Pixels = px(240.);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_pane_is_a_third_of_the_content_and_adapts() {
        assert_eq!(right_pane_w(px(1200.), RIGHT_RATIO), px(400.));
        assert_eq!(right_pane_w(px(2100.), RIGHT_RATIO), px(700.));
        // Narrow windows keep the panel's min width.
        assert_eq!(right_pane_w(px(600.), RIGHT_RATIO), RIGHT_MIN_W);
        // A wide share never squeezes the diff under its min width.
        assert_eq!(right_pane_w(px(1000.), 0.6), px(1000.) - DIFF_MIN_W);
    }

    #[test]
    fn dragged_share_round_trips_and_clamps() {
        let r = right_ratio(px(1200.), px(480.));
        assert!((r - 0.4).abs() < 1e-6);
        assert_eq!(right_pane_w(px(1200.), r), px(480.));
        assert_eq!(right_ratio(px(1200.), px(10.)), RIGHT_RATIO_MIN);
        assert_eq!(right_ratio(px(0.), px(10.)), RIGHT_RATIO);
    }
}
