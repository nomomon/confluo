//! CI status icon of the open PR: hover lists the jobs, click opens the full checks popover.

use gpui::{deferred, div, prelude::*, AnyElement, ClickEvent, Context, MouseDownEvent, SharedString};

use crate::app::GitViewer;
use crate::features::shared::ci::{ci_status, status_icon, CiCard};
use crate::state::DetailState;
use crate::ui::{motion, theme};

impl GitViewer {
    /// Nothing at all when the PR reports no checks.
    pub(super) fn render_ci(&self, d: &DetailState, id: &str, cx: &mut Context<Self>) -> Option<AnyElement> {
        let info = d.detail.as_ref().map(|x| x.checks.clone());
        let status = ci_status(&d.summary.checks, info.as_ref())?;
        let failing = info.as_ref().map(|c| c.count(gv_engine::CheckStatus::Failed) as u32).unwrap_or(d.summary.checks.failed);
        let open = self.review.checks_open;
        let icon = div()
            .id(SharedString::from(format!("ci-{id}")))
            .flex()
            .items_center()
            .gap(theme::SPACE_1)
            .px(theme::SPACE_1)
            .rounded(theme::RADIUS_SM)
            .cursor_pointer()
            .hover(|s| s.bg(theme::surface_hover()))
            .hoverable_tooltip(CiCard::tooltip(d.summary.checks, info))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.review.checks_open = !this.review.checks_open;
                cx.notify()
            }))
            .child(status_icon(&format!("ci-{id}"), status))
            .when(failing > 0, |e| e.child(div().text_size(theme::text_xs()).text_color(theme::danger()).child(format!("{failing} failing"))));
        let popover = open.then(|| {
            let panel = div()
                .id("checks-popover")
                .w(theme::SPACE_16 * 7.)
                .max_h(theme::SPACE_16 * 8.)
                .overflow_y_scroll()
                .rounded(theme::RADIUS_LG)
                .border_1()
                .border_color(theme::border())
                .bg(theme::overlay())
                .shadow_lg()
                .occlude()
                .on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.review.checks_open = false;
                    cx.notify()
                }))
                .child(self.render_checks_card(d, cx));
            deferred(div().absolute().top(theme::SPACE_8).right_0().child(motion::pop_in(panel, "checks-popover-pop"))).with_priority(2)
        });
        Some(div().relative().flex_shrink_0().child(icon).children(popover).into_any_element())
    }
}
