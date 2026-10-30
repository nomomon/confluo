//! Review mode: the PR takes the whole content area. Top bar, then file tree | diff | right panel,
//! the side panes resizable and collapsible (Cmd+Shift+E, Cmd+Shift+I).

use gpui::{div, prelude::*, Context, Div, Pixels};

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
        let center = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(theme::space_0())
            .h_full()
            .bg(self.surfaces().content())
            .px(theme::space_4())
            .pt(theme::space_3())
            .when_some(d.error.clone(), |c, e| c.child(ErrorText::new(e)))
            .child(self.render_code(d, cx))
            .children(
                self.render_code_menus(d, cx)
                    .filter(|_| matches!(self.layer, Some(crate::app::Layer::LinesAt(_)))),
            );
        let body = div()
            .flex()
            .flex_1()
            .min_h(theme::space_0())
            .relative()
            .children(self.panel_frame(
                Pane::Tree,
                self.side_pane(false).child(self.render_file_tree(d, cx)),
                cx,
            ))
            .child(center)
            .children(self.panel_frame(
                Pane::Right,
                self.side_pane(true).child(self.render_right_panel(d, cx)),
                cx,
            ))
            .children(self.panel_edge(Pane::Tree, cx))
            .children(self.panel_edge(Pane::Right, cx));
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_review_bar(d, cx))
            .child(body)
            .children(self.render_quick_open(d, cx))
    }

    /// Width of one right panel page (the panel's content box, inside its 1px left border) as laid out
    /// this frame: the Overview / Conversation pager steps by exactly this.
    pub(crate) fn right_page_w(&self) -> f32 {
        (f32::from(layout::right_pane_w(
            self.review_content_w(),
            self.review.right_ratio,
        )) - 1.)
            .max(1.)
    }

    /// Window width minus the open file tree: what the diff and the right panel share.
    pub(crate) fn review_content_w(&self) -> Pixels {
        self.viewport_w
            - if self.review.tree_open {
                self.review.tree_w
            } else {
                gpui::px(0.)
            }
    }

    fn side_pane(&self, right: bool) -> Div {
        let pane = div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(self.surfaces().panel())
            .border_color(theme::border_subtle());
        if right {
            pane.border_l_1()
        } else {
            pane.border_r_1()
        }
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
