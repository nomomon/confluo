//! Centered placeholder shown when a list has nothing to display.
//!
//! Extra lines or actions go in as children, stacked under the hint.

use gpui::{div, prelude::*, AnyElement, App, SharedString, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct EmptyState {
    title: SharedString,
    hint: SharedString,
    children: Vec<AnyElement>,
}

impl EmptyState {
    pub fn new(title: impl Into<SharedString>, hint: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            hint: hint.into(),
            children: vec![],
        }
    }
}

impl ParentElement for EmptyState {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
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
            .child(div().text_color(theme::text_muted()).child(self.title))
            .child(
                div()
                    .text_size(theme::text_xs())
                    .text_color(theme::text_subtle())
                    .child(self.hint),
            )
            .when(!self.children.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(theme::SPACE_2)
                        .pt(theme::SPACE_3)
                        .children(self.children),
                )
            })
    }
}
