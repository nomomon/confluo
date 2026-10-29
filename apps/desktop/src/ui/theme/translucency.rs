//! Panel and card fills derived from the user's appearance settings.

use gpui::Hsla;

use super::palette::{light, zinc};
use super::runtime::is_light;
use super::tokens::bg;

/// Appearance inputs: panel opacity (0..=1), window tint and base window alpha.
#[derive(Clone, Copy)]
pub struct Translucency {
    pub panel_opacity: f32,
    pub tint: Hsla,
    /// Alpha of the tinted window background (1.0 when opaque).
    pub base_alpha: f32,
}

impl Translucency {
    pub fn new(panel_opacity: f32, tint: Option<Hsla>, base_alpha: f32) -> Self {
        Self { panel_opacity, tint: tint.unwrap_or_else(bg), base_alpha }
    }

    /// Tinted window background behind everything. In light mode the (dark)
    /// tint keeps its hue but is lifted to a soft page grey.
    pub fn window_bg(&self) -> Hsla {
        let tint = if is_light() { Hsla { s: self.tint.s.min(0.2), l: light::PAGE.l - self.tint.l * 0.1, ..self.tint } } else { self.tint };
        Hsla { a: self.base_alpha, ..tint }
    }

    /// Sidebar and detail panel fill.
    pub fn panel(&self) -> Hsla {
        let base = if is_light() { light::PANEL } else { zinc::Z900 };
        Hsla { a: self.panel_opacity, ..base }
    }

    /// Card fill inside panels: a touch lighter (darker in light mode), more see-through.
    pub fn card(&self) -> Hsla {
        let base = if is_light() { light::CARD } else { zinc::Z800 };
        Hsla { a: self.panel_opacity * 0.6, ..base }
    }
}
