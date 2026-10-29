//! Rounded status pill tinted with a single color.

use gpui::{div, prelude::*, App, Hsla, SharedString, Window};

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
            .px_2()
            .py_0p5()
            .rounded_full()
            .text_xs()
            .bg(self.color.opacity(0.15))
            .text_color(self.color)
            .child(self.label)
    }
}
