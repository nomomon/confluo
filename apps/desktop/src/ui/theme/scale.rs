//! Spacing, radius and type scales. Spacing follows Tailwind's 4px steps.

// Full scales are part of the design-system API even where unused.
#![allow(dead_code)]

use gpui::{px, FontWeight, Pixels};

pub fn space_0() -> Pixels {
    zoomed(0.)
}
pub fn space_0p5() -> Pixels {
    zoomed(2.)
}
pub fn space_1() -> Pixels {
    zoomed(4.)
}
pub fn space_1p5() -> Pixels {
    zoomed(6.)
}
pub fn space_2() -> Pixels {
    zoomed(8.)
}
pub fn space_2p5() -> Pixels {
    zoomed(10.)
}
pub fn space_3() -> Pixels {
    zoomed(12.)
}
pub fn space_4() -> Pixels {
    zoomed(16.)
}
pub fn space_5() -> Pixels {
    zoomed(20.)
}
pub fn space_6() -> Pixels {
    zoomed(24.)
}
pub fn space_7() -> Pixels {
    zoomed(28.)
}
pub fn space_8() -> Pixels {
    zoomed(32.)
}
pub fn space_10() -> Pixels {
    zoomed(40.)
}
pub fn space_12() -> Pixels {
    zoomed(48.)
}
pub fn space_14() -> Pixels {
    zoomed(56.)
}
pub fn space_16() -> Pixels {
    zoomed(64.)
}

pub const RADIUS_SM: Pixels = px(6.);
pub const RADIUS_MD: Pixels = px(8.);
pub const RADIUS_LG: Pixels = px(12.);
pub const RADIUS_XL: Pixels = px(16.);
pub const RADIUS_FULL: Pixels = px(9999.);

/// Icon sizes: inline chevrons, buttons and row state.
pub fn icon_sm() -> Pixels {
    zoomed(12.)
}
pub fn icon_md() -> Pixels {
    zoomed(14.)
}
pub fn icon_lg() -> Pixels {
    zoomed(16.)
}

/// Spacing and icon sizes follow the UI zoom (Cmd+= / Cmd+-), like the type scale.
fn zoomed(base: f32) -> Pixels {
    px(base * super::runtime::typography().zoom)
}

/// Type scale. Sizes follow the user's text step (Settings, 13/14/15 body).
fn sized(base: f32) -> Pixels {
    let t = super::runtime::typography();
    px((base + t.step) * t.zoom)
}
/// Tiny labels inside avatars and badges.
pub fn text_2xs() -> Pixels {
    sized(10.)
}
/// Shortcut hint pills (hold-Cmd overlay).
pub fn text_hint() -> Pixels {
    sized(11.)
}
pub fn text_xs() -> Pixels {
    sized(12.)
}
/// Body default.
pub fn text_sm() -> Pixels {
    sized(13.)
}
pub fn text_base() -> Pixels {
    sized(14.)
}
/// List row titles (13.5, medium weight).
pub fn text_md() -> Pixels {
    sized(13.5)
}
pub fn text_lg() -> Pixels {
    sized(16.)
}
/// Detail title.
pub fn text_xl() -> Pixels {
    sized(18.)
}
/// Page titles.
pub fn text_2xl() -> Pixels {
    sized(22.)
}

/// Body line height multiplier.
pub const LINE_HEIGHT: f32 = 1.4;
/// Rendered markdown paragraphs and lists, and the markdown editor that replaces them.
pub const MARKDOWN_LINE_HEIGHT: f32 = 1.55;

pub const WEIGHT_NORMAL: FontWeight = FontWeight::NORMAL;
pub const WEIGHT_MEDIUM: FontWeight = FontWeight::MEDIUM;
pub const WEIGHT_SEMIBOLD: FontWeight = FontWeight::SEMIBOLD;
pub const WEIGHT_BOLD: FontWeight = FontWeight::BOLD;

/// Density-driven rhythm. Settings > Appearance > Density picks Compact (default, first value)
/// or Comfortable (second value); everything that sets the app's breathing room reads these.
fn dense(compact: f32, comfortable: f32) -> Pixels {
    px(if super::runtime::typography().compact {
        compact
    } else {
        comfortable
    })
}
/// Height of primary controls (tabs, search, inputs).
pub fn control_h() -> Pixels {
    dense(32., 36.)
}
/// Minimum height of a two-line list row.
pub fn row_min_h() -> Pixels {
    dense(52., 62.)
}
/// Vertical padding inside list rows.
pub fn row_pad_y() -> Pixels {
    dense(8., 12.)
}
/// Horizontal padding inside list rows.
pub fn row_pad_x() -> Pixels {
    dense(12., 16.)
}
/// Gap between the title line and the meta line of a row.
pub fn row_line_gap() -> Pixels {
    dense(4., 6.)
}
/// Height of a key/value row inside a card.
pub fn kv_row_h() -> Pixels {
    dense(36., 44.)
}
/// Side padding of the list column (detail closed / open).
pub fn page_pad_x() -> Pixels {
    dense(32., 48.)
}
pub fn page_pad_x_narrow() -> Pixels {
    dense(20., 24.)
}
/// Space above a section header.
pub fn section_gap() -> Pixels {
    dense(20., 28.)
}
/// Space between a section header and its card.
pub fn section_header_gap() -> Pixels {
    dense(8., 12.)
}
/// Gap between stacked cards (detail pane) and between page blocks.
pub fn card_gap() -> Pixels {
    dense(12., 16.)
}
/// Padding inside cards and the detail pane.
pub fn card_pad() -> Pixels {
    dense(14., 16.)
}
pub fn pane_pad() -> Pixels {
    dense(20., 24.)
}
/// Vertical padding of nav, menu and palette items.
pub fn item_pad_y() -> Pixels {
    dense(5., 7.)
}
/// Inset of a highlighted row from its card edge; the row radius is the card radius minus this.
pub const LIST_INSET: Pixels = px(4.);
/// Section card radius and the inset row radius inside it.
pub const LIST_RADIUS: Pixels = RADIUS_LG;
pub const LIST_ITEM_RADIUS: Pixels = RADIUS_MD;
/// Width of the key column in key/value rows.
pub const KV_KEY_W: Pixels = px(88.);

/// Width of a slider in a settings row.
pub const SETTINGS_SLIDER_W: Pixels = px(220.);

/// Minimum width of dropdown menus.
pub const MENU_MIN_W: Pixels = px(180.);
/// Content that is not on the server yet (a comment being sent).
pub const OPACITY_PENDING: f32 = 0.6;
