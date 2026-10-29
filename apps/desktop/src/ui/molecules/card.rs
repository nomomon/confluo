//! Bordered translucent card container.

use gpui::{div, prelude::*, AnyElement, App, Div, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct Card {
    base: Div,
    opacity: f32,
}

impl Card {
    /// `opacity` is the panel opacity from settings; cards render slightly lighter.
    pub fn new(opacity: f32) -> Self {
        Self { base: div(), opacity }
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements)
    }
}

impl RenderOnce for Card {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(theme::border())
            .bg(theme::card(self.opacity * 0.8))
    }
}
