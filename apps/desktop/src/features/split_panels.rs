//! Dividers and collapsed edges of the side panels (`state::split`): every panel draws through
//! `panel_frame`, which takes the drawn width while a drag or spring moves it and clips the content at
//! its resting width, so nothing reflows mid-animation.

use gpui::{
    div, prelude::*, px, AnyElement, Context, Div, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Stateful,
};

use crate::app::GitViewer;
use crate::features::layout;
use crate::state::Pane;
use crate::ui::split::EDGE_W;
use crate::ui::theme;

fn pane_id(p: Pane, edge: bool) -> &'static str {
    match (p, edge) {
        (Pane::Tree, false) => "tree-resize",
        (Pane::Right, false) => "right-resize",
        (Pane::Sidebar, false) => "sidebar-resize",
        (Pane::Preview, false) => "preview-resize",
        (Pane::Tree, true) => "tree-edge",
        (Pane::Right, true) => "right-edge",
        (Pane::Sidebar, true) => "sidebar-edge",
        (Pane::Preview, true) => "preview-edge",
    }
}

/// Panels on the left edge put their divider on the right.
fn on_left(p: Pane) -> bool {
    matches!(p, Pane::Tree | Pane::Sidebar)
}

impl GitViewer {
    /// The divider of panel `p` (absolute, straddling its inner edge).
    pub(crate) fn panel_divider(&self, p: Pane, cx: &mut Context<Self>) -> impl IntoElement {
        let h = div()
            .id(pane_id(p, false))
            .absolute()
            .top_0()
            .h_full()
            .w(layout::RESIZE_HANDLE_W)
            .cursor_col_resize()
            .hover(|s| s.bg(theme::border()));
        let h = if on_left(p) {
            h.right(-layout::RESIZE_HANDLE_W / 2.)
        } else {
            h.left(-layout::RESIZE_HANDLE_W / 2.)
        };
        h.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _, cx| this.start_panel_drag(p, cx)),
        )
    }

    /// A collapsed panel's thin grabbable edge (drag out to open); None while it is open or moving.
    pub(crate) fn panel_edge(&self, p: Pane, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.panel_open(p) || self.panel_shown(p).is_some() {
            return None;
        }
        let e = div()
            .id(pane_id(p, true))
            .absolute()
            .top_0()
            .h_full()
            .w(px(EDGE_W))
            .cursor_col_resize()
            .hover(|s| s.bg(theme::border()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| this.start_panel_drag(p, cx)),
            );
        Some(if on_left(p) { e.left_0() } else { e.right_0() }.into_any_element())
    }

    /// `content` in panel `p`'s frame: drawn width from the drag or spring, content kept at least at its
    /// minimum (clipped, not squeezed). None when the panel is collapsed and still.
    pub(crate) fn panel_frame(
        &self,
        p: Pane,
        content: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let shown = self.panel_shown(p);
        if !self.panel_open(p) && shown.is_none() {
            return None;
        }
        let l = self.panel_limits(p);
        let rest = self.panel_rest_w(p);
        let w = shown.unwrap_or(rest);
        let inner_w = w.max(l.min);
        let anchored = if on_left(p) {
            div().absolute().top_0().left_0()
        } else {
            div().absolute().top_0().right_0()
        };
        Some(
            div()
                .relative()
                .flex_shrink_0()
                .h_full()
                .w(px(w))
                .child(
                    div()
                        .size_full()
                        .overflow_hidden()
                        .relative()
                        .child(anchored.h_full().w(px(inner_w)).child(content)),
                )
                .child(self.panel_divider(p, cx)),
        )
    }

    /// Root listeners while a divider or edge is dragged.
    pub(crate) fn panel_drag_handlers(
        &self,
        root: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        root.on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, window, cx| {
            this.move_panel_drag(ev.position, window.viewport_size().width, cx)
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _: &MouseUpEvent, _, cx| this.end_panel_drag(cx)),
        )
    }
}
