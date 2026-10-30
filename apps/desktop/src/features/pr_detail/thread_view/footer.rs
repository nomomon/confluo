//! Thread footer: a one-line Reply field (the inline composer in the diff, the pinned composer in the
//! Conversation) and Resolve thread / Reopen thread on the right.

use gpui::{div, prelude::*, ClickEvent, Context, Div, Focusable, SharedString};
use gv_engine::Thread;

use super::super::comment::Place;
use super::section;
use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::{Button, ButtonVariant};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn thread_footer(
        &self,
        t: &Thread,
        d: &DetailState,
        place: Place,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        if t.id == "pending" {
            return None;
        }
        let foot = section().px(theme::space_3()).py(theme::space_2());
        if place == Place::Code && d.code.inline.reply_to.as_deref() == Some(t.id.as_str()) {
            return Some(foot.child(self.inline_field("Reply", cx)));
        }
        let replying = place == Place::Activity && d.reply_to.as_deref() == Some(t.id.as_str());
        let tid = t.id.clone();
        let field = div()
            .id(SharedString::from(format!("reply-field-{}", t.id)))
            .flex_1()
            .min_w(theme::space_0())
            .px(theme::space_2p5())
            .py(theme::space_1p5())
            .rounded(theme::RADIUS_SM)
            .border_1()
            .border_color(if replying {
                theme::focus_ring()
            } else {
                theme::border()
            })
            .bg(theme::container_bg())
            .text_size(theme::text_sm())
            .text_color(theme::text_subtle())
            .cursor_text()
            .hover(|s| s.border_color(theme::text_subtle()))
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| match place {
                    Place::Code => this.open_inline_field(Some(tid.clone()), None, window, cx),
                    Place::Activity => {
                        if let Some(d) = this.detail.as_mut() {
                            d.reply_to = Some(tid.clone());
                            d.editing = None;
                        }
                        window.focus(&this.composer.focus_handle(cx));
                        cx.notify()
                    }
                }),
            )
            .child(if replying {
                "Replying in the composer below"
            } else {
                "Reply..."
            });
        let resolve = t.resolved.map(|resolved| {
            let tid = t.id.clone();
            Button::new(
                format!("thread-resolve-{}", t.id),
                if resolved {
                    "Reopen thread"
                } else {
                    "Resolve thread"
                },
            )
            .variant(ButtonVariant::Outline)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.set_thread_resolved(tid.clone(), !resolved, cx)
            }))
        });
        Some(
            foot.flex()
                .items_center()
                .gap(theme::space_2())
                .child(field)
                .children(resolve),
        )
    }
}
