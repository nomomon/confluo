//! Dark palette (Zed/Linear-like). Panel backgrounds take an opacity from settings.

use gpui::{hsla, Hsla};

pub fn text() -> Hsla {
    hsla(0., 0., 0.92, 1.)
}
pub fn muted() -> Hsla {
    hsla(240. / 360., 0.05, 0.62, 1.)
}
pub fn faint() -> Hsla {
    hsla(240. / 360., 0.05, 0.45, 1.)
}
pub fn border() -> Hsla {
    hsla(0., 0., 1., 0.08)
}
pub fn field() -> Hsla {
    hsla(0., 0., 1., 0.05)
}
pub fn hover() -> Hsla {
    hsla(0., 0., 1., 0.06)
}
pub fn selected() -> Hsla {
    hsla(0., 0., 1., 0.10)
}
pub fn accent() -> Hsla {
    hsla(230. / 360., 0.85, 0.66, 1.)
}
pub fn green() -> Hsla {
    hsla(142. / 360., 0.55, 0.55, 1.)
}
pub fn red() -> Hsla {
    hsla(0., 0.7, 0.62, 1.)
}
pub fn yellow() -> Hsla {
    hsla(42. / 360., 0.9, 0.6, 1.)
}
pub fn purple() -> Hsla {
    hsla(270. / 360., 0.6, 0.68, 1.)
}
/// Base panel color; multiply alpha by the configured opacity.
pub fn panel(opacity: f32) -> Hsla {
    hsla(240. / 360., 0.08, 0.10, opacity)
}
pub fn card(opacity: f32) -> Hsla {
    hsla(240. / 360., 0.08, 0.14, opacity)
}

/// Parse `#rrggbb` into a color.
pub fn parse_hex(s: &str) -> Option<Hsla> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(gpui::rgb(v).into())
}
