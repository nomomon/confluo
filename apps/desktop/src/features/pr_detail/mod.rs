//! Right-hand detail panel: header, Summary or Activity body, floating tab pill.

mod activity;
mod checkout;
mod checks;
mod header;
mod overview;
mod tabs;

pub(crate) use checkout::checkout_command;

use gpui::{div, prelude::*, Context, MouseButton, MouseDownEvent};

use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::{DetailState, DetailTab};
use crate::ui::atoms::ErrorText;
use crate::ui::{motion, theme};

impl GitViewer {
    pub(crate) fn render_detail(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let col = div()
            .id("detail-scroll")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(theme::SPACE_0)
            .overflow_y_scroll()
            .gap(theme::SPACE_3)
            .pb(theme::SPACE_4)
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
            .hover(|s| s.bg(theme::BORDER))
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
            .border_color(theme::BORDER_SUBTLE)
            .pt(layout::TITLEBAR_INSET)
            .px(theme::SPACE_4)
            .gap(theme::SPACE_2)
            .child(col)
            .child(div().flex().justify_center().pb(theme::SPACE_2).child(self.render_detail_tabs(d, cx)))
            .when(d.tab == DetailTab::Activity, |p| p.child(self.render_composer(d, cx)))
            .when(d.tab == DetailTab::Summary, |p| p.pb(theme::SPACE_2))
            .child(handle);
        motion::slide_in_right(panel, "detail-panel", theme::SPACE_6)
    }
}
