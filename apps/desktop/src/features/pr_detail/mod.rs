//! Right-hand detail panel: header, Summary, Code or Activity body, floating tab pill.

mod activity;
mod checkout;
mod checks;
mod code;
mod header;
mod overview;
mod tabs;

pub(crate) use checkout::checkout_command;

use gpui::{div, prelude::*, Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Stateful, Div};

use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::{DetailState, DetailTab};
use crate::ui::atoms::ErrorText;
use crate::ui::{motion, theme};

impl GitViewer {
    /// Root listeners while the resize handle is dragged.
    pub(crate) fn resize_handlers(&self, root: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        root.on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, window, cx| {
            let w = window.viewport_size().width - ev.position.x;
            this.detail_w = w.clamp(layout::DETAIL_MIN_W, layout::DETAIL_MAX_W);
            cx.notify()
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _: &MouseUpEvent, _, cx| {
                this.resizing = false;
                cx.notify()
            }),
        )
    }

    pub(crate) fn render_detail(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let code = d.tab == DetailTab::Code;
        let col = div()
            .id("detail-scroll")
            .track_scroll(&self.detail_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(theme::SPACE_0)
            .when(!code, |c| c.overflow_y_scroll())
            .gap(theme::SPACE_5)
            .when(!code, |c| c.pb(theme::SPACE_8))
            .child(self.render_detail_header(d, cx))
            .when_some(d.error.clone(), |col, e| col.child(ErrorText::new(e)))
            .child(self.render_detail_body(d, cx));
        let handle = div()
            .id("detail-resize")
            .absolute()
            .top_0()
            .left_0()
            .h_full()
            .w(layout::RESIZE_HANDLE_W)
            .cursor_col_resize()
            .hover(|s| s.bg(theme::border()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.resizing = true;
                    cx.notify()
                }),
            );
        let panel = div()
            .flex()
            .flex_col()
            .w(self.detail_w)
            .flex_shrink_0()
            .h_full()
            .bg(self.surfaces().panel())
            .border_l_1()
            .border_color(theme::border_subtle())
            .pt(layout::TITLEBAR_INSET)
            .px(theme::SPACE_6)
            .gap(theme::SPACE_3)
            .child(col)
            .child(div().flex().justify_center().pb(theme::SPACE_3).child(self.render_detail_tabs(d, cx)))
            .when(d.tab == DetailTab::Activity, |p| p.child(self.render_composer(d, cx)))
            .when(d.tab == DetailTab::Summary, |p| p.pb(theme::SPACE_3))
            .child(handle);
        motion::slide_in_right(panel, "detail-panel", theme::SPACE_6)
    }
}
