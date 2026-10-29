//! Glyph icon in a fixed square, tinted with one color.

use gpui::{div, prelude::*, App, Hsla, SharedString, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct Icon {
    glyph: SharedString,
    color: Hsla,
}

impl Icon {
    pub fn new(glyph: impl Into<SharedString>) -> Self {
        Self { glyph: glyph.into(), color: theme::TEXT_MUTED }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().flex().flex_shrink_0().items_center().justify_center().size(theme::SPACE_4).text_size(theme::TEXT_XS).text_color(self.color).child(self.glyph)
    }
}
