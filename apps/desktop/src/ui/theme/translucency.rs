//! Panel and card fills derived from the user's appearance settings.
//!
//! Readability floors: over a translucent window the desktop behind can be
//! anything. Dark fills never drop below `DARK_WINDOW_MIN` / `DARK_PANEL_MIN`
//! alpha, so text keeps >= 4.5:1 even with a pure white window behind; light
//! fills never drop below `LIGHT_WINDOW_MIN` (pure black behind).

use gpui::{Hsla, Rgba};

use super::palette::{light, zinc};
use super::runtime::is_light;

const DARK_WINDOW_MIN: f32 = 0.9;
const DARK_PANEL_MIN: f32 = 0.82;
const LIGHT_WINDOW_MIN: f32 = 0.86;
const LIGHT_PANEL_MIN: f32 = 0.8;
/// How strongly the tint colors the window background, and the panels.
const TINT_WINDOW: f32 = 0.16;
const TINT_PANEL: f32 = 0.07;

/// Appearance inputs: panel opacity (0..=1), window tint and base window alpha.
#[derive(Clone, Copy)]
pub struct Translucency {
    pub panel_opacity: f32,
    /// Saturated tint color, or `None` for neutral.
    pub tint: Option<Hsla>,
    /// Alpha of the tinted window background (1.0 when opaque).
    pub base_alpha: f32,
}

/// Mix `a` toward `b` by `t` in sRGB.
pub fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let (a, b): (Rgba, Rgba) = (a.into(), b.into());
    let l = |x: f32, y: f32| x + (y - x) * t;
    Rgba { r: l(a.r, b.r), g: l(a.g, b.g), b: l(a.b, b.b), a: 1. }.into()
}

impl Translucency {
    pub fn new(panel_opacity: f32, tint: Option<Hsla>, base_alpha: f32) -> Self {
        Self { panel_opacity, tint, base_alpha }
    }

    fn tinted(&self, base: Hsla, strength: f32) -> Hsla {
        match self.tint {
            Some(t) => mix(base, t, if is_light() { strength * 0.6 } else { strength }),
            None => base,
        }
    }

    /// Tinted window background behind everything.
    pub fn window_bg(&self) -> Hsla {
        let (base, floor) = if is_light() { (light::PAGE, LIGHT_WINDOW_MIN) } else { (zinc::Z950, DARK_WINDOW_MIN) };
        Hsla { a: self.base_alpha.max(floor), ..self.tinted(base, TINT_WINDOW) }
    }

    /// Sidebar and detail panel fill.
    pub fn panel(&self) -> Hsla {
        let (base, floor) = if is_light() { (light::PANEL, LIGHT_PANEL_MIN) } else { (zinc::Z900, DARK_PANEL_MIN) };
        Hsla { a: floor + (1. - floor) * self.panel_opacity.clamp(0., 1.), ..self.tinted(base, TINT_PANEL) }
    }

    /// Card fill inside panels: a touch lighter (darker in light mode), more see-through.
    pub fn card(&self) -> Hsla {
        let base = if is_light() { light::CARD } else { zinc::Z800 };
        Hsla { a: 0.35 + 0.35 * self.panel_opacity, ..self.tinted(base, TINT_PANEL) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lum(c: Rgba) -> f32 {
        let f = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
        0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
    }

    fn over(top: Hsla, under: Rgba) -> Rgba {
        let t: Rgba = top.into();
        let m = |x: f32, y: f32| x * t.a + y * (1. - t.a);
        Rgba { r: m(t.r, under.r), g: m(t.g, under.g), b: m(t.b, under.b), a: 1. }
    }

    fn contrast(a: Rgba, b: Rgba) -> f32 {
        let (x, y) = (lum(a) + 0.05, lum(b) + 0.05);
        x.max(y) / x.min(y)
    }

    #[test]
    fn dark_text_reads_over_white() {
        super::super::set_light(false);
        let s = Translucency::new(0.0, None, 0.35);
        let white = Rgba { r: 1., g: 1., b: 1., a: 1. };
        let page = over(s.window_bg(), white);
        assert!(contrast(super::super::text().into(), page) >= 4.5);
        let panel = over(s.panel(), page);
        assert!(contrast(super::super::text_muted().into(), panel) >= 4.5);
    }
}
