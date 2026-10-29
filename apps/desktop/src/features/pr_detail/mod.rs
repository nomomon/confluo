//! Right-hand detail panel for the selected PR.

mod activity;
mod checkout;
mod checks;
mod header;
mod overview;
mod tabs;

use gpui::{div, prelude::*, px, Context};

use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::ErrorText;
use crate::ui::molecules::Card;
use crate::ui::theme;

impl GitViewer {
    pub(crate) fn render_detail(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let op = self.settings.panel_opacity;
        let col = div()
            .id("detail-scroll")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_3()
            .pb_4()
            .child(self.render_detail_header(d, cx))
            .child(overview::overview_card(&d.summary, op))
            .child(self.render_checks_card(d, cx))
            .child(self.render_checkout_card(d, cx))
            .when_some(d.error.clone(), |col, e| col.child(ErrorText::new(e)))
            .child(self.render_detail_tabs(d, cx))
            .child(Card::new(op).child(self.render_detail_body(d, cx)));

        div()
            .flex()
            .flex_col()
            .w(px(540.))
            .flex_shrink_0()
            .h_full()
            .bg(theme::panel(op))
            .border_l_1()
            .border_color(theme::border())
            .pt(px(36.))
            .px_4()
            .gap_3()
            .child(col)
            .child(self.render_composer(d, cx))
    }
}
