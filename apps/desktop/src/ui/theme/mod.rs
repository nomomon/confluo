//! Design tokens: palette, semantic colors, spacing/radius/type scales and translucency.
//!
//! See `docs/design-system.md`.

pub mod palette;
mod scale;
mod tokens;
mod translucency;

pub use scale::*;
pub use tokens::*;
pub use translucency::Translucency;

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
