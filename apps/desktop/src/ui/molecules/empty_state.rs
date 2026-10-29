//! Centered placeholder shown when a list has nothing to display.

use gpui::{div, prelude::*, App, SharedString, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct EmptyState {
    title: SharedString,
    hint: SharedString,
}

impl EmptyState {
    pub fn new(title: impl Into<SharedString>, hint: impl Into<SharedString>) -> Self {
        Self { title: title.into(), hint: hint.into() }
    }
}

impl RenderOnce for EmptyState {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .py(theme::SPACE_12 + theme::SPACE_8)
            .gap(theme::SPACE_1)
            .child(div().text_color(theme::TEXT_MUTED).child(self.title))
            .child(div().text_size(theme::TEXT_XS).text_color(theme::TEXT_SUBTLE).child(self.hint))
    }
}
