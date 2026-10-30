//! Form field like shadcn `Field`: label, optional muted description and a control.
//!
//! Stacked: `Field::new("Source", input)`. Settings row (text left, control right):
//! `Field::new("Hide bot PRs", switch).description("Renovate, Dependabot").horizontal()`.

use gpui::{div, prelude::*, AnyElement, App, SharedString, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct Field {
    label: SharedString,
    description: Option<SharedString>,
    control: AnyElement,
    horizontal: bool,
}

impl Field {
    pub fn new(label: impl Into<SharedString>, control: impl IntoElement) -> Self {
        Self { label: label.into(), description: None, control: control.into_any_element(), horizontal: false }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Label and description on the left, control on the right, padded for a `Card::rows()`.
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }
}

impl RenderOnce for Field {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let desc = self.description.map(|d| div().text_size(theme::text_xs()).text_color(theme::text_muted()).child(d));
        if self.horizontal {
            let text = div().flex_1().min_w(theme::SPACE_0).flex().flex_col().gap(theme::SPACE_0P5).child(div().text_size(theme::text_sm()).child(self.label)).children(desc);
            return div().flex().items_center().gap(theme::SPACE_4).px(theme::SPACE_4).py(theme::SPACE_3).child(text).child(div().flex().flex_shrink_0().child(self.control));
        }
        div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_1P5)
            .child(div().text_size(theme::text_xs()).font_weight(theme::WEIGHT_MEDIUM).text_color(theme::text_muted()).child(self.label))
            .child(self.control)
            .children(desc)
    }
}
