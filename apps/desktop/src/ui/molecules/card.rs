//! Bordered translucent card container.
//!
//! `Card::new(surfaces).child(..)` pads its content; `.rows()` lays children
//! flush and splits them with hairline dividers (key/value lists).

use gpui::{div, prelude::*, AnyElement, App, Hsla, Window};

use crate::ui::theme::{self, Translucency};

#[derive(IntoElement)]
pub struct Card {
    fill: Hsla,
    rows: bool,
    children: Vec<AnyElement>,
}

impl Card {
    /// Cards take their fill from the appearance-derived translucency.
    pub fn new(surfaces: Translucency) -> Self {
        Self { fill: surfaces.card(), rows: false, children: vec![] }
    }

    /// Flush rows separated by dividers instead of padded content.
    pub fn rows(mut self) -> Self {
        self.rows = true;
        self
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Card {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let base = div().flex().flex_col().flex_shrink_0().rounded(theme::RADIUS_XL).border_1().border_color(theme::border_subtle()).bg(self.fill);
        if self.rows {
            base.overflow_hidden().children(self.children.into_iter().enumerate().map(|(i, c)| div().when(i > 0, |d| d.border_t_1().border_color(theme::border_subtle())).child(c)))
        } else {
            base.gap(theme::SPACE_3).p(theme::SPACE_4).children(self.children)
        }
    }
}
