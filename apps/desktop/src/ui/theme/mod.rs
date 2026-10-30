//! Design tokens: palette, semantic colors, spacing/radius/type scales and translucency.
//!
//! See `docs/design-system.md`.

mod light;
pub mod palette;
mod runtime;
mod scale;
mod syntax;
mod tokens;
mod translucency;

pub use light::{set_light_variant, sidebar_inset};
pub use runtime::{is_light, mono_font, set_light, set_typography, ui_font, zoom, Typography};
pub use scale::*;
pub use syntax::*;
pub use tokens::*;
pub use translucency::{mix, Translucency};

use gpui::Hsla;

/// Parse `#rrggbb` into a color.
pub fn parse_hex(s: &str) -> Option<Hsla> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(gpui::rgb(v).into())
}
