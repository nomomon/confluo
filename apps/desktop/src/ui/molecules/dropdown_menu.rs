//! Floating menu panel; position it with `Styled` methods such as `.top()` and `.right()`.
//!
//! `DropdownMenu::new("detail-more").top(..).right(..).on_dismiss(..).child(NavItem::new(..))`

use gpui::{div, prelude::*, AnyElement, App, Div, MouseDownEvent, SharedString, StyleRefinement, Window};

use crate::ui::{motion, theme, MouseDownHandler};

#[derive(IntoElement)]
pub struct DropdownMenu {
    id: SharedString,
    base: Div,
    children: Vec<AnyElement>,
    on_dismiss: Option<MouseDownHandler>,
}

impl DropdownMenu {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self { id: id.into(), base: div(), children: vec![], on_dismiss: None }
    }

    /// Called on a mouse press outside the menu.
    pub fn on_dismiss(mut self, f: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(f));
        self
    }
}

impl Styled for DropdownMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for DropdownMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for DropdownMenu {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let panel = div()
            .min_w(theme::MENU_MIN_W)
            .text_size(theme::text_sm())
            .p(theme::SPACE_1)
            .flex()
            .flex_col()
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(theme::BORDER)
            .bg(theme::OVERLAY)
            .shadow_lg()
            .children(self.children)
            .when_some(self.on_dismiss, |el, f| el.on_mouse_down_out(f));
        self.base.absolute().occlude().child(motion::pop_in(panel, self.id))
    }
}
