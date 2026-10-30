//! CI status icon of the open PR: hover lists the jobs, click opens the full checks popover.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, MouseDownEvent, SharedString};

use crate::app::{GitViewer, Layer};
use crate::features::shared::ci::{ci_status, status_icon, CiCard};
use crate::state::DetailState;
use crate::ui::molecules::{Placement, Popover};
use crate::ui::theme;

impl GitViewer {
    /// Nothing at all when the PR reports no checks.
    pub(super) fn render_ci(
        &self,
        d: &DetailState,
        id: &str,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let info = d.detail.as_ref().map(|x| x.checks.clone());
        let status = ci_status(&d.summary.checks, info.as_ref())?;
        let failing = info
            .as_ref()
            .map(|c| c.count(gv_engine::CheckStatus::Failed) as u32)
            .unwrap_or(d.summary.checks.failed);
        // The Overview row spells the summary out ("2 failing, 5 passed"); bars show only the failing count.
        let words = (id == "overview")
            .then(|| crate::features::shared::ci::ci_words(&d.summary.checks))
            .filter(|w| !w.is_empty());
        let open = self.layer == Some(Layer::Checks);
        let icon = div()
            .id(SharedString::from(format!("ci-{id}")))
            .flex()
            .items_center()
            .gap(theme::space_1())
            .px(theme::space_1())
            .rounded(theme::RADIUS_SM)
            .cursor_pointer()
            .hover(|s| s.bg(theme::surface_hover()))
            // The hover card steps aside while the popover (or any other layer) is open.
            .when(!self.layer_open(), |e| {
                e.hoverable_tooltip(CiCard::tooltip(d.summary.checks, info))
            })
            .on_click(
                cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_layer(Layer::Checks, cx)),
            )
            .child(status_icon(&format!("ci-{id}"), status))
            .when_some(words.clone(), |e, w| {
                e.child(
                    div()
                        .text_size(theme::text_xs())
                        .text_color(theme::text_muted())
                        .child(w),
                )
            })
            .when(failing > 0 && words.is_none(), |e| {
                e.child(
                    div()
                        .text_size(theme::text_xs())
                        .text_color(theme::danger())
                        .child(format!("{failing} failing")),
                )
            });
        let popover = open.then(|| {
            let panel = div()
                .id("checks-popover")
                .w(theme::space_16() * 7.)
                .max_h(theme::space_16() * 8.)
                .map(crate::ui::scroll::y_only)
                .rounded(theme::RADIUS_LG)
                .border_1()
                .border_color(theme::border())
                .bg(theme::overlay())
                .shadow_lg()
                .child(self.render_checks_card(d, cx));
            Popover::new("checks-popover-pop")
                .placement(Placement::BottomEnd)
                .on_dismiss(cx.listener(|this, _: &MouseDownEvent, _, cx| this.dismiss_layer(cx)))
                .child(panel)
        });
        Some(
            div()
                .relative()
                .flex_shrink_0()
                .child(icon)
                .children(popover)
                .into_any_element(),
        )
    }
}
