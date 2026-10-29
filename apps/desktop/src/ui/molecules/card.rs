//! Bordered translucent card container.

use gpui::{div, prelude::*, AnyElement, App, Div, Hsla, Window};

use crate::ui::theme::{self, Translucency};

#[derive(IntoElement)]
pub struct Card {
    base: Div,
    fill: Hsla,
}

impl Card {
    /// Cards take their fill from the appearance-derived translucency.
    pub fn new(surfaces: Translucency) -> Self {
        Self { base: div(), fill: surfaces.card() }
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
            .gap(theme::SPACE_2)
            .p(theme::SPACE_3)
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(theme::BORDER_SUBTLE)
            .bg(self.fill)
    }
}
