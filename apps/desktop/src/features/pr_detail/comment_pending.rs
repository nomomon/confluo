//! Meta line of a comment in the outbox (`state::outbox`): "Sending..." with a spinner, or
//! "Couldn't send" in red (reason on hover) with Retry, Edit and Discard.

use gpui::{div, prelude::*, ClickEvent, Context, Div, SharedString};
use gv_engine::Post;

use crate::app::GitViewer;
use crate::state::outbox::{Pending, Status};
use crate::ui::atoms::{tooltip, Button, ButtonVariant, Spinner};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn pending_meta(&self, p: &Pending, cx: &mut Context<Self>) -> Div {
        let row = div().flex().items_center().gap(theme::space_1p5());
        let cid = p.cid.clone();
        match &p.status {
            Status::Failed(why) => {
                let act = |name: &str, label: &'static str| {
                    Button::new(SharedString::from(format!("{name}-{cid}")), label)
                        .variant(ButtonVariant::Link)
                };
                let (c1, c2, c3) = (cid.clone(), cid.clone(), cid.clone());
                row.child(
                    div()
                        .id(SharedString::from(format!("failed-{cid}")))
                        .text_color(theme::danger())
                        .tooltip(tooltip(why.clone()))
                        .child("Couldn't send"),
                )
                .child(act("retry", "Retry").on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.retry_post(c1.clone(), cx)),
                ))
                // An inline comment's anchor cannot be reopened in the composer.
                .when(!matches!(p.post, Post::Line(_)), |e| {
                    e.child(act("edit", "Edit").on_click(cx.listener(
                        move |this, _: &ClickEvent, window, cx| {
                            this.edit_post(c2.clone(), window, cx)
                        },
                    )))
                })
                .child(act("discard", "Discard").on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| this.discard_post(c3.clone(), cx),
                )))
            }
            _ => row
                .text_color(theme::text_subtle())
                .child(Spinner::new(SharedString::from(format!("sending-{cid}"))))
                .child("Sending\u{2026}"),
        }
    }
}
