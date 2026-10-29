//! Floating menu panel; position it with `Styled` methods such as `.top()` and `.right()`.

use gpui::{div, prelude::*, px, AnyElement, App, Div, StyleRefinement, Window};

use crate::ui::{motion, theme};

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
        let el = self.base
            .absolute()
            .w(px(220.)).text_size(theme::TEXT_SM)
            .p(theme::SPACE_1)
            .flex()
            .flex_col()
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(theme::BORDER)
            .bg(theme::OVERLAY)
            .shadow_lg()
            .occlude();
        motion::pop_in(el, "dropdown-menu")
    }
}
