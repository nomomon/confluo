//! Rounded status pill tinted with a single color.

use gpui::{div, prelude::*, App, Hsla, SharedString, Window};
use crate::ui::theme;

#[derive(IntoElement)]
pub struct Pill {
    label: SharedString,
    color: Hsla,
}

impl Pill {
    pub fn new(label: impl Into<SharedString>, color: Hsla) -> Self {
        Self { label: label.into(), color }
    }
}

impl RenderOnce for Pill {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex_shrink_0()
            .px(theme::SPACE_2)
            .py(theme::SPACE_0P5)
            .rounded(theme::RADIUS_FULL)
            .text_size(theme::TEXT_XS)
            .bg(theme::tint_bg(self.color))
            .text_color(self.color)
            .child(self.label)
    }
}
