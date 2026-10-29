//! Round initial-letter avatar.

use gpui::{div, prelude::*, App, SharedString, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct Avatar {
    name: SharedString,
}

impl Avatar {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self { name: name.into() }
    }
}

impl RenderOnce for Avatar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let initial: String = self.name.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(theme::SPACE_4)
            .rounded(theme::RADIUS_FULL)
            .bg(theme::SURFACE_ACTIVE)
            .text_color(theme::TEXT_MUTED)
            .text_size(theme::TEXT_2XS)
            .font_weight(theme::WEIGHT_SEMIBOLD)
            .child(initial)
    }
}
