//! Small square badge with short bold text on a tinted background.

use gpui::{div, prelude::*, px, App, FontWeight, Hsla, SharedString, Window};

#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    color: Hsla,
}

impl Badge {
    pub fn new(label: impl Into<SharedString>, color: Hsla) -> Self {
        Self { label: label.into(), color }
    }
}

impl RenderOnce for Badge {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(px(26.))
            .rounded_md()
            .bg(self.color.opacity(0.14))
            .text_color(self.color)
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .child(self.label)
    }
}
