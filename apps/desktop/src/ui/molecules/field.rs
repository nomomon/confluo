//! Form field: a muted label stacked above its control.

use gpui::{div, prelude::*, AnyElement, App, SharedString, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct Field {
    label: SharedString,
    control: AnyElement,
}

impl Field {
    pub fn new(label: impl Into<SharedString>, control: impl IntoElement) -> Self {
        Self { label: label.into(), control: control.into_any_element() }
    }
}

impl RenderOnce for Field {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().flex().flex_col().gap_1().child(div().text_xs().text_color(theme::muted()).child(self.label)).child(self.control)
    }
}
