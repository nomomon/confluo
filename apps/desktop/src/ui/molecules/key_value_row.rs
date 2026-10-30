//! Label on the left, arbitrary value on the right; one 52px row of a `Card::rows()`.

use gpui::{div, prelude::*, AnyElement, App, SharedString, Window};

use crate::ui::theme;

#[derive(IntoElement)]
#[allow(dead_code)] // kit API
pub struct KeyValueRow {
    key: SharedString,
    value: AnyElement,
}

#[allow(dead_code)] // kit API
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
            .gap(theme::SPACE_4)
            .min_h(theme::kv_row_h())
            .px(theme::card_pad())
            .text_size(theme::text_sm())
            .child(div().w(theme::KV_KEY_W).flex_shrink_0().text_color(theme::text_muted()).child(self.key))
            .child(self.value)
    }
}
