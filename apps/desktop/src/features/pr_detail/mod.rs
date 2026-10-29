//! Right-hand detail panel for the selected PR.

mod activity;
mod checkout;
mod checks;
mod header;
mod overview;
mod tabs;

use gpui::{div, prelude::*, Context};

use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::ErrorText;
use crate::ui::molecules::Card;
use crate::ui::theme;
use crate::features::shared::layout;

impl GitViewer {
    pub(crate) fn render_detail(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let tr = self.surfaces();
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
            .child(overview::overview_card(&d.summary, tr))
            .child(self.render_checks_card(d, cx))
            .child(self.render_checkout_card(d, cx))
            .when_some(d.error.clone(), |col, e| col.child(ErrorText::new(e)))
            .child(self.render_detail_tabs(d, cx))
            .child(Card::new(tr).child(self.render_detail_body(d, cx)));

        div()
            .flex()
            .flex_col()
            .w(layout::DETAIL_W)
            .flex_shrink_0()
            .h_full()
            .bg(tr.panel())
            .border_l_1()
            .border_color(theme::BORDER_SUBTLE)
            .pt(layout::TITLEBAR_INSET)
            .px(theme::SPACE_4)
            .gap(theme::SPACE_3)
            .child(col)
            .child(self.render_composer(d, cx))
    }
}
