//! Review mode: the PR takes the whole content area. Top bar, then file tree | diff | right panel,
//! the side panes resizable and collapsible (Cmd+Shift+E, Cmd+Shift+I).

use gpui::{
    div, prelude::*, Context, Div, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Stateful,
};

use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::{DetailState, Pane};
use crate::ui::atoms::ErrorText;
use crate::ui::theme;

impl GitViewer {
    pub(crate) fn render_review(
        &self,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let r = &self.review;
        let tree_w = r.tree_w.min(
            (self.viewport_w - layout::DIFF_MIN_W - layout::RIGHT_MIN_W).max(layout::TREE_MIN_W),
        );
        let right_w = layout::right_pane_w(self.review_content_w(), r.right_ratio);
        let center = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(theme::SPACE_0)
            .h_full()
            .bg(self.surfaces().content())
            .px(theme::SPACE_4)
            .pt(theme::SPACE_3)
            .when_some(d.error.clone(), |c, e| c.child(ErrorText::new(e)))
            .child(self.render_code(d, cx))
            .children(
                self.render_code_menus(d, cx)
                    .filter(|_| matches!(self.layer, Some(crate::app::Layer::LinesAt(_)))),
            );
        let body = div()
            .flex()
            .flex_1()
            .min_h(theme::SPACE_0)
            .when(r.tree_open, |b| {
                b.child(
                    self.side_pane(tree_w, false)
                        .child(self.render_file_tree(d, cx))
                        .child(self.resize_handle(Pane::Tree, cx)),
                )
            })
            .child(center)
            .when(r.right_open, |b| {
                b.child(
                    self.side_pane(right_w, true)
                        .child(self.render_right_panel(d, cx))
                        .child(self.resize_handle(Pane::Right, cx)),
                )
            });
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_review_bar(d, cx))
            .child(body)
            .children(self.render_quick_open(d, cx))
    }

    /// Window width minus the open file tree: what the diff and the right panel share.
    fn review_content_w(&self) -> Pixels {
        self.viewport_w
            - if self.review.tree_open {
                self.review.tree_w
            } else {
                gpui::px(0.)
            }
    }

    fn side_pane(&self, w: Pixels, right: bool) -> Div {
        let pane = div()
            .relative()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .w(w)
            .h_full()
            .bg(self.surfaces().panel())
            .border_color(theme::border_subtle());
        if right {
            pane.border_l_1()
        } else {
            pane.border_r_1()
        }
    }

    fn resize_handle(&self, pane: Pane, cx: &mut Context<Self>) -> impl IntoElement {
        let h = div()
            .id(if pane == Pane::Tree {
                "tree-resize"
            } else {
                "right-resize"
            })
            .absolute()
            .top_0()
            .h_full()
            .w(layout::RESIZE_HANDLE_W)
            .cursor_col_resize()
            .hover(|s| s.bg(theme::border()));
        let h = if pane == Pane::Tree {
            h.right(-layout::RESIZE_HANDLE_W / 2.)
        } else {
            h.left(-layout::RESIZE_HANDLE_W / 2.)
        };
        h.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                this.review.resizing = Some(pane);
                cx.notify()
            }),
        )
    }

    /// Root listeners while a pane's resize handle is dragged.
    pub(crate) fn resize_handlers(
        &self,
        root: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        root.on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, window, cx| {
            match this.review.resizing {
                Some(Pane::Tree) => {
                    this.review.tree_w = ev.position.x.clamp(layout::TREE_MIN_W, layout::TREE_MAX_W)
                }
                Some(Pane::Right) => {
                    let content = this.review_content_w();
                    this.review.right_ratio =
                        layout::right_ratio(content, window.viewport_size().width - ev.position.x)
                }
                None => return,
            }
            cx.notify()
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _: &MouseUpEvent, _, cx| {
                this.review.resizing = None;
                this.settings.review_tree_w = f32::from(this.review.tree_w);
                this.settings.review_right_ratio = this.review.right_ratio;
                this.save_settings(cx);
                cx.notify()
            }),
        )
    }

    pub(crate) fn toggle_tree(&mut self, cx: &mut Context<Self>) {
        self.review.tree_open = !self.review.tree_open;
        cx.notify()
    }

    pub(crate) fn toggle_right(&mut self, cx: &mut Context<Self>) {
        self.review.right_open = !self.review.right_open;
        cx.notify()
    }
}
