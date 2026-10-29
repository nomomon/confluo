//! Semantic color tokens. Features use these, never raw palette values.
//!
//! Surfaces are translucent white overlays so they sit well on the tinted,
//! possibly blurred window background.

use gpui::Hsla;

use super::palette::{zinc, AMBER, BLACK, BLUE, GREEN, PURPLE, RED, WHITE};

const fn alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a, ..c }
}

/// Opaque app background (also the fallback tint).
pub const BG: Hsla = zinc::Z950;
/// Resting fill for inputs, buttons and inline code.
pub const SURFACE: Hsla = alpha(WHITE, 0.04);
/// Fill on pointer hover.
pub const SURFACE_HOVER: Hsla = alpha(WHITE, 0.07);
/// Fill for pressed or selected items.
pub const SURFACE_ACTIVE: Hsla = alpha(WHITE, 0.10);
/// Default 1px border.
pub const BORDER: Hsla = alpha(WHITE, 0.08);
/// Hairline for dividers and card outlines.
pub const BORDER_SUBTLE: Hsla = alpha(WHITE, 0.05);
/// Primary text.
pub const TEXT: Hsla = zinc::Z100;
/// Secondary text: metadata, labels.
pub const TEXT_MUTED: Hsla = zinc::Z400;
/// Tertiary text: hints, timestamps, placeholders.
pub const TEXT_SUBTLE: Hsla = zinc::Z500;
/// Links and interactive emphasis.
pub const ACCENT: Hsla = BLUE;
/// Keyboard focus outline.
pub const FOCUS_RING: Hsla = alpha(BLUE, 0.6);
/// Open, added, passed.
pub const SUCCESS: Hsla = GREEN;
/// Removed, failed, closed.
pub const DANGER: Hsla = RED;
/// Pending, needs you, starred.
pub const WARNING: Hsla = AMBER;
/// Merged.
pub const MERGED: Hsla = PURPLE;
/// Floating layers (dialogs, menus): near-opaque so text stays legible.
pub const OVERLAY: Hsla = alpha(zinc::Z900, 0.97);
/// Dimmed backdrop behind modal dialogs.
pub const SCRIM: Hsla = alpha(BLACK, 0.45);

/// Low-alpha fill of a semantic color, for pills and banners.
pub fn tint_bg(c: Hsla) -> Hsla {
    c.opacity(0.14)
}
