//! Label on the left, arbitrary value on the right.

use gpui::{div, prelude::*, px, AnyElement, App, SharedString, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct KeyValueRow {
    key: SharedString,
    value: AnyElement,
}

impl KeyValueRow {
    pub fn new(key: impl Into<SharedString>, value: impl IntoElement) -> Self {
        Self { key: key.into(), value: value.into_any_element() }
    }
}

impl RenderOnce for KeyValueRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().w(px(80.)).text_xs().text_color(theme::muted()).child(self.key))
            .child(self.value)
    }
}
