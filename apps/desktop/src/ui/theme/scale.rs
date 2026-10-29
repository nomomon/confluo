//! Spacing, radius and type scales. Spacing follows Tailwind's 4px steps.

// Full scales are part of the design-system API even where unused.
#![allow(dead_code)]

use gpui::{px, FontWeight, Pixels};

pub const SPACE_0: Pixels = px(0.);
pub const SPACE_0P5: Pixels = px(2.);
pub const SPACE_1: Pixels = px(4.);
pub const SPACE_1P5: Pixels = px(6.);
pub const SPACE_2: Pixels = px(8.);
pub const SPACE_2P5: Pixels = px(10.);
pub const SPACE_3: Pixels = px(12.);
pub const SPACE_4: Pixels = px(16.);
pub const SPACE_5: Pixels = px(20.);
pub const SPACE_6: Pixels = px(24.);
pub const SPACE_7: Pixels = px(28.);
pub const SPACE_8: Pixels = px(32.);
pub const SPACE_10: Pixels = px(40.);
pub const SPACE_12: Pixels = px(48.);
pub const SPACE_14: Pixels = px(56.);
pub const SPACE_16: Pixels = px(64.);

pub const RADIUS_SM: Pixels = px(6.);
pub const RADIUS_MD: Pixels = px(8.);
pub const RADIUS_LG: Pixels = px(12.);
pub const RADIUS_XL: Pixels = px(16.);
pub const RADIUS_FULL: Pixels = px(9999.);

/// Icon sizes: inline chevrons, buttons and row state.
pub const ICON_SM: Pixels = px(12.);
pub const ICON_MD: Pixels = px(14.);
pub const ICON_LG: Pixels = px(16.);

/// Type scale. Sizes follow the user's text step (Settings, 13/14/15 body).
fn sized(base: f32) -> Pixels {
    px(base + super::runtime::typography().step)
}
/// Tiny labels inside avatars and badges.
pub fn text_2xs() -> Pixels {
    sized(10.)
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
/// List row titles.
pub fn text_md() -> Pixels {
    sized(15.)
}
pub fn text_lg() -> Pixels {
    sized(16.)
}
pub fn text_xl() -> Pixels {
    sized(20.)
}
/// Page titles.
pub fn text_2xl() -> Pixels {
    sized(28.)
}

pub const WEIGHT_NORMAL: FontWeight = FontWeight::NORMAL;
pub const WEIGHT_MEDIUM: FontWeight = FontWeight::MEDIUM;
pub const WEIGHT_SEMIBOLD: FontWeight = FontWeight::SEMIBOLD;
pub const WEIGHT_BOLD: FontWeight = FontWeight::BOLD;


/// Rhythm: height of primary controls (tabs, search, filter chips).
pub const CONTROL_H: Pixels = px(40.);
/// Rhythm: minimum height of a two-line list row (16px vertical padding).
pub const ROW_MIN_H: Pixels = px(68.);
/// Rhythm: height of a key/value row inside a card.
pub const KV_ROW_H: Pixels = px(52.);
/// Width of the key column in key/value rows.
pub const KV_KEY_W: Pixels = px(88.);

/// Minimum width of dropdown menus.
pub const MENU_MIN_W: Pixels = px(180.);
