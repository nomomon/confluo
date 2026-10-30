//! Semantic color tokens. Features use these, never raw palette values.
//!
//! Each token is a function over the live light/dark palette, so switching
//! the theme applies on the next frame. Surfaces are translucent overlays
//! (white on dark, black on light) so they sit well on the tinted, possibly
//! blurred window background.

use gpui::Hsla;

use super::light::surfaces as light_surfaces;
use super::palette::{dark, light, zinc, BLACK, WHITE};
use super::runtime::is_light;

const fn alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a, ..c }
}

/// One full set of semantic colors.
pub(super) struct Colors {
    bg: Hsla,
    surface: Hsla,
    surface_hover: Hsla,
    surface_active: Hsla,
    border: Hsla,
    border_subtle: Hsla,
    text: Hsla,
    text_muted: Hsla,
    text_subtle: Hsla,
    accent: Hsla,
    success: Hsla,
    danger: Hsla,
    warning: Hsla,
    merged: Hsla,
    overlay: Hsla,
    scrim: Hsla,
}

const DARK: Colors = Colors {
    bg: zinc::Z950,
    surface: alpha(WHITE, 0.04),
    surface_hover: alpha(WHITE, 0.07),
    surface_active: alpha(WHITE, 0.10),
    border: alpha(WHITE, 0.08),
    border_subtle: alpha(WHITE, 0.05),
    text: zinc::Z100,
    text_muted: zinc::Z400,
    text_subtle: zinc::Z500,
    accent: dark::BLUE,
    success: dark::GREEN,
    danger: dark::RED,
    warning: dark::AMBER,
    merged: dark::PURPLE,
    overlay: alpha(zinc::Z900, 1.0),
    scrim: alpha(BLACK, 0.45),
};

const LIGHT: Colors = Colors {
    bg: WHITE,
    surface: alpha(BLACK, 0.04),
    surface_hover: alpha(BLACK, 0.05),
    surface_active: alpha(BLACK, 0.08),
    border: alpha(BLACK, 0.10),
    border_subtle: alpha(BLACK, 0.07),
    text: light::TEXT,
    text_muted: light::TEXT_MUTED,
    text_subtle: light::TEXT_SUBTLE,
    accent: light::BLUE,
    success: light::GREEN,
    danger: light::RED,
    warning: light::AMBER,
    merged: light::PURPLE,
    overlay: alpha(WHITE, 1.0),
    scrim: alpha(BLACK, 0.18),
};

fn c() -> &'static Colors {
    if is_light() {
        &LIGHT
    } else {
        &DARK
    }
}

/// Opaque app background (also the fallback tint).
pub fn bg() -> Hsla {
    c().bg
}
/// Resting fill for inputs, buttons and inline code.
pub fn surface() -> Hsla {
    c().surface
}
/// Fill on pointer hover.
pub fn surface_hover() -> Hsla {
    c().surface_hover
}
/// Fill for pressed or selected items.
pub fn surface_active() -> Hsla {
    c().surface_active
}
/// Default 1px border.
pub fn border() -> Hsla {
    c().border
}
/// Card and list section outline: the hairline in dark, per light variant (none when flat).
pub fn card_border() -> Hsla {
    if is_light() {
        alpha(BLACK, light_surfaces().card_border)
    } else {
        c().border_subtle
    }
}
/// Hairline for dividers and card outlines.
pub fn border_subtle() -> Hsla {
    c().border_subtle
}
/// Primary text.
pub fn text() -> Hsla {
    c().text
}
/// Secondary text: metadata, labels.
pub fn text_muted() -> Hsla {
    c().text_muted
}
/// Tertiary text: hints, timestamps, placeholders.
pub fn text_subtle() -> Hsla {
    c().text_subtle
}
/// Links and interactive emphasis.
pub fn accent() -> Hsla {
    c().accent
}
/// Keyboard focus outline.
pub fn focus_ring() -> Hsla {
    c().accent.opacity(0.6)
}
/// Open, added, passed.
pub fn success() -> Hsla {
    c().success
}
/// Removed, failed, closed.
pub fn danger() -> Hsla {
    c().danger
}
/// Pending, needs you, starred.
pub fn warning() -> Hsla {
    c().warning
}
/// Merged.
pub fn merged() -> Hsla {
    c().merged
}
/// Floating layers (dialogs, menus): near-opaque so text stays legible.
pub fn overlay() -> Hsla {
    c().overlay
}
/// Dimmed backdrop behind modal dialogs.
pub fn scrim() -> Hsla {
    c().scrim
}

/// Low-alpha fill of a semantic color, only where meaning needs it (8% light, 10% dark).
pub fn tint_bg(c: Hsla) -> Hsla {
    c.opacity(if is_light() { 0.08 } else { 0.10 })
}

/// Flat container fill (comment threads): white in light, the surface overlay in dark.
pub fn container_bg() -> Hsla {
    if is_light() {
        WHITE
    } else {
        c().surface
    }
}
/// Solid disc behind every avatar, so transparent logos stay visible.
pub fn avatar_bg() -> Hsla {
    if is_light() {
        WHITE
    } else {
        zinc::Z800
    }
}
/// Hairline ring around avatars.
pub fn avatar_ring() -> Hsla {
    if is_light() {
        alpha(BLACK, 0.10)
    } else {
        alpha(WHITE, 0.10)
    }
}
/// Inline code chip fill (GitLab-like: #F0F0F2 light, white 8% dark).
pub fn code_inline_bg() -> Hsla {
    if is_light() {
        alpha(BLACK, 0.055)
    } else {
        alpha(WHITE, 0.08)
    }
}
