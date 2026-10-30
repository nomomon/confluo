//! Menu opened at the pointer like shadcn `ContextMenu`; same rows as `DropdownMenu`.
//! Store the right-click position and render this while it is open.
//!
//! `ContextMenu::new("row-menu", event.position).on_dismiss(..).child(MenuItem::new(..))`

use gpui::{
    anchored, deferred, div, prelude::*, AnyElement, App, MouseDownEvent, Pixels, Point,
    SharedString, Window,
};

use crate::ui::{layer, motion, theme, MouseDownHandler};

#[derive(IntoElement)]
pub struct ContextMenu {
    id: SharedString,
    position: Point<Pixels>,
    children: Vec<AnyElement>,
    on_dismiss: Option<MouseDownHandler>,
}

impl ContextMenu {
    pub fn new(id: impl Into<SharedString>, position: Point<Pixels>) -> Self {
        Self {
            id: id.into(),
            position,
            children: vec![],
            on_dismiss: None,
        }
    }

    pub fn on_dismiss(
        mut self,
        f: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Box::new(f));
        self
    }
}

impl ParentElement for ContextMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let panel = layer::track(div())
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
            .occlude()
            .children(self.children)
            .when_some(self.on_dismiss, |el, f| el.on_mouse_down_out(f));
        deferred(
            anchored()
                .position(self.position)
                .snap_to_window_with_margin(theme::space_2())
                .child(motion::pop_in(panel, self.id)),
        )
        .with_priority(2)
    }
}
