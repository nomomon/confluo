//! One review thread: location, resolved flag, comments and reply link.

use gpui::{div, prelude::*, ClickEvent, Context, Focusable, FontWeight};
use gv_engine::{Comment, Thread};

use crate::app::GitViewer;
use crate::features::shared::relative_time;
use crate::state::DetailState;
use crate::ui::atoms::{Button, ButtonVariant};
use crate::ui::molecules::markdown::markdown;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_thread(&self, t: &Thread, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let mut tc = div().flex().flex_col().gap_2().p_2().rounded_md().border_1().border_color(theme::border());
        if let Some(p) = t.comments.first().and_then(|c| c.position.as_ref()) {
            tc = tc.child(
                div()
                    .text_xs()
                    .font_family(theme::MONO_FONT)
                    .text_color(theme::muted())
                    .child(format!("{}{}", p.path, p.line.map(|l| format!(":{l}")).unwrap_or_default())),
            );
        }
        if t.resolved == Some(true) {
            tc = tc.child(div().text_xs().text_color(theme::green()).child("Resolved"));
        }
        for c in &t.comments {
            tc = tc.child(self.render_comment(c, d, cx));
        }
        if t.id != "pending" {
            let tid = t.id.clone();
            tc = tc.child(Button::new(format!("reply-{}", t.id), "Reply").variant(ButtonVariant::Link).on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| {
                    if let Some(d) = this.detail.as_mut() {
                        d.reply_to = Some(tid.clone());
                        d.editing = None;
                    }
                    window.focus(&this.composer.focus_handle(cx));
                    cx.notify()
                },
            )));
        }
        tc
    }

    fn render_comment(&self, c: &Comment, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let mine = d.me.as_deref().is_some_and(|m| m.eq_ignore_ascii_case(&c.author.username));
        let cid = c.id.clone();
        let cbody = c.body.clone();
        let when = if c.id == "pending" { "posting...".into() } else { relative_time(c.created_at) };
        let mut head = div()
            .flex()
            .items_center()
            .gap_2()
            .text_xs()
            .child(div().font_weight(FontWeight::SEMIBOLD).child(c.author.username.clone()))
            .child(div().text_color(theme::faint()).child(when))
            .child(div().flex_1());
        if mine && c.id != "pending" {
            head = head.child(Button::new(format!("edit-{}", c.id), "Edit").variant(ButtonVariant::Link).on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| {
                    if let Some(d) = this.detail.as_mut() {
                        d.editing = Some(cid.clone());
                        d.reply_to = None;
                    }
                    this.composer.update(cx, |i, cx| i.set_text(cbody.clone(), cx));
                    window.focus(&this.composer.focus_handle(cx));
                    cx.notify()
                },
            )));
        }
        div().flex().flex_col().gap_1().child(head).child(div().flex().flex_col().gap_1().children(markdown(&c.body)))
    }
}
