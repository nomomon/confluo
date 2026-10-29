//! Small square badge with short bold text on a tinted background.

use gpui::{div, prelude::*, App, Hsla, SharedString, Window};
use crate::ui::theme;

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
            .size(theme::SPACE_6)
            .rounded(theme::RADIUS_MD)
            .bg(theme::tint_bg(self.color))
            .text_color(self.color)
            .text_size(theme::text_xs())
            .font_weight(theme::WEIGHT_BOLD)
            .child(self.label)
    }
}
