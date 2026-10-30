//! Small floating pill of actions, e.g. over a text or line selection.
//!
//! `FloatingToolbar::new("sel-bar").animate(!keyboard).child(Button::new(..))`. The caller positions
//! it (usually absolute inside `deferred(..)` so it paints above later siblings). It occludes what
//! is beneath and swallows mouse downs so clicks never reach the content under it.

use gpui::{div, prelude::*, AnyElement, App, ElementId, MouseButton, Window};

use crate::ui::{motion, theme};

#[derive(IntoElement)]
pub struct FloatingToolbar {
    id: ElementId,
    animate: bool,
    children: Vec<AnyElement>,
}

impl FloatingToolbar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            animate: true,
            children: vec![],
        }
    }

    /// Pop in on mount (default). Pass false for keyboard-driven appearances.
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = on;
        self
    }
}

impl ParentElement for FloatingToolbar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for FloatingToolbar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let pill = div()
            .id(self.id.clone())
            .occlude()
            .flex()
            .items_center()
            .gap(theme::SPACE_0P5)
            .p(theme::SPACE_0P5)
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_lg()
            .font_family(theme::ui_font())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(self.children);
        if self.animate {
            motion::pop_in(pill, self.id)
        } else {
            pill.into_any_element()
        }
    }
}
