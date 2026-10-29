//! Floating menu panel; position it with `Styled` methods such as `.top()` and `.right()`.

use gpui::{div, prelude::*, px, AnyElement, App, Div, StyleRefinement, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct DropdownMenu {
    base: Div,
}

impl DropdownMenu {
    pub fn new() -> Self {
        Self { base: div() }
    }
}

impl Styled for DropdownMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for DropdownMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements)
    }
}

impl RenderOnce for DropdownMenu {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .absolute()
            .w(px(220.))
            .p_1()
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(theme::border())
            .bg(theme::card(0.97))
            .shadow_lg()
            .occlude()
    }
}
