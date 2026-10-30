//! Menu panel under (or above) its trigger, through `Popover`: render it inside a `relative()`
//! wrapper around the trigger button.
//!
//! `DropdownMenu::new("detail-more").placement(Placement::BottomEnd).on_dismiss(..).child(MenuItem::new(..))`

use gpui::{div, prelude::*, AnyElement, App, MouseDownEvent, SharedString, Window};

use super::{Placement, Popover};
use crate::ui::{theme, MouseDownHandler};

#[derive(IntoElement)]
pub struct DropdownMenu {
    id: SharedString,
    placement: Placement,
    children: Vec<AnyElement>,
    on_dismiss: Option<MouseDownHandler>,
}

#[allow(dead_code)] // kit API
impl DropdownMenu {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            placement: Placement::BottomEnd,
            children: vec![],
            on_dismiss: None,
        }
    }

    /// Default `BottomEnd` (right-aligned under the trigger).
    pub fn placement(mut self, p: Placement) -> Self {
        self.placement = p;
        self
    }

    /// Called on a mouse press outside the menu.
    pub fn on_dismiss(
        mut self,
        f: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Box::new(f));
        self
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
            .p(theme::space_1())
            .flex()
            .flex_col()
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_lg()
            .children(self.children);
        let pop = Popover::new(self.id).placement(self.placement).child(panel);
        match self.on_dismiss {
            Some(f) => pop.on_dismiss(f),
            None => pop,
        }
    }
}
