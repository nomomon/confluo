//! Panel and card fills derived from the user's appearance settings.

use gpui::Hsla;

use super::palette::zinc;
use super::tokens::BG;

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
        Self { panel_opacity, tint: tint.unwrap_or(BG), base_alpha }
    }

    /// Tinted window background behind everything.
    pub fn window_bg(&self) -> Hsla {
        Hsla { a: self.base_alpha, ..self.tint }
    }

    /// Sidebar and detail panel fill.
    pub fn panel(&self) -> Hsla {
        Hsla { a: self.panel_opacity, ..zinc::Z900 }
    }

    /// Card fill inside panels: a touch lighter, more see-through.
    pub fn card(&self) -> Hsla {
        Hsla { a: self.panel_opacity * 0.6, ..zinc::Z800 }
    }
}
