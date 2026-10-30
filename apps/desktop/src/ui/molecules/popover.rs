//! Floating panel anchored to its trigger: menus, popovers and pickers all go through this.
//!
//! Render it as a child of a `relative()` wrapper around the trigger. It paints through
//! `deferred(anchored())`, so it escapes the parent's clipping and stacking (it draws above every
//! region at the window root), flips to the other side when it would leave the window and then
//! shifts to stay on screen. While open it is the window's one floating layer (`ui::layer`).
//!
//! `Popover::new("more").placement(Placement::BottomEnd).on_dismiss(..).child(panel)`

use gpui::{
    anchored, deferred, div, point, prelude::*, AnyElement, App, Corner, MouseDownEvent, Pixels,
    SharedString, Window,
};

use crate::ui::{layer, motion, theme, MouseDownHandler};

/// Where the panel sits relative to the trigger.
#[allow(dead_code)] // kit API
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Placement {
    /// Below, left edges aligned.
    #[default]
    BottomStart,
    /// Below, right edges aligned (buttons near the right edge).
    BottomEnd,
    /// Above, left edges aligned.
    TopStart,
    /// Above, right edges aligned.
    TopEnd,
}

impl Placement {
    /// The panel corner pinned to the trigger corner of the same name, and the gap direction.
    fn corner(self) -> (Corner, f32) {
        match self {
            Self::BottomStart => (Corner::TopLeft, 1.),
            Self::BottomEnd => (Corner::TopRight, 1.),
            Self::TopStart => (Corner::BottomLeft, -1.),
            Self::TopEnd => (Corner::BottomRight, -1.),
        }
    }
}

#[derive(IntoElement)]
pub struct Popover {
    id: SharedString,
    placement: Placement,
    offset: Pixels,
    children: Vec<AnyElement>,
    on_dismiss: Option<MouseDownHandler>,
    animate: bool,
}

#[allow(dead_code)] // kit API
impl Popover {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            placement: Placement::default(),
            offset: theme::SPACE_1,
            children: vec![],
            on_dismiss: None,
            animate: true,
        }
    }

    pub fn placement(mut self, p: Placement) -> Self {
        self.placement = p;
        self
    }

    /// Gap between trigger and panel (default `SPACE_1`).
    pub fn offset(mut self, gap: Pixels) -> Self {
        self.offset = gap;
        self
    }

    /// Called on a mouse press outside the panel.
    pub fn on_dismiss(
        mut self,
        f: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Box::new(f));
        self
    }

    /// Skip the entrance (keyboard-driven opens).
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = on;
        self
    }
}

impl ParentElement for Popover {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Popover {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (corner, dir) = self.placement.corner();
        let panel = layer::track(div().occlude().children(self.children))
            .when_some(self.on_dismiss, |el, f| el.on_mouse_down_out(f));
        let panel = if self.animate {
            motion::pop_in(panel, self.id).into_any_element()
        } else {
            panel.into_any_element()
        };
        // A zero-size marker at the trigger corner; `anchored` pins the panel's matching corner to it.
        let marker = div().absolute().size_0();
        let marker = match self.placement {
            Placement::BottomStart => marker.top_full().left_0(),
            Placement::BottomEnd => marker.top_full().right_0(),
            Placement::TopStart => marker.top_0().left_0(),
            Placement::TopEnd => marker.top_0().right_0(),
        };
        deferred(
            marker.child(
                anchored()
                    .anchor(corner)
                    .offset(point(gpui::px(0.), self.offset * dir))
                    .child(panel),
            ),
        )
        .with_priority(2)
    }
}
