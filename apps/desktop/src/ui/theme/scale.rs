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
pub const SPACE_8: Pixels = px(32.);
pub const SPACE_10: Pixels = px(40.);
pub const SPACE_12: Pixels = px(48.);

pub const RADIUS_SM: Pixels = px(6.);
pub const RADIUS_MD: Pixels = px(8.);
pub const RADIUS_LG: Pixels = px(12.);
pub const RADIUS_XL: Pixels = px(16.);
pub const RADIUS_FULL: Pixels = px(9999.);

/// Tiny labels inside avatars and badges.
pub const TEXT_2XS: Pixels = px(10.);
pub const TEXT_XS: Pixels = px(12.);
pub const TEXT_SM: Pixels = px(13.);
pub const TEXT_BASE: Pixels = px(14.);
pub const TEXT_LG: Pixels = px(16.);
pub const TEXT_XL: Pixels = px(20.);
/// Page titles.
pub const TEXT_2XL: Pixels = px(26.);

pub const WEIGHT_NORMAL: FontWeight = FontWeight::NORMAL;
pub const WEIGHT_MEDIUM: FontWeight = FontWeight::MEDIUM;
pub const WEIGHT_SEMIBOLD: FontWeight = FontWeight::SEMIBOLD;
pub const WEIGHT_BOLD: FontWeight = FontWeight::BOLD;

/// System UI font.
pub const UI_FONT: &str = ".SystemUIFont";
/// Monospace font for branches, commands and SHAs.
pub const MONO_FONT: &str = "Menlo";
