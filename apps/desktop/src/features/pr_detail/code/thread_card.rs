//! Inline review thread under its diff line: comments, reply field, edit, resolve, and a one-line resolved state.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, Div};
use gv_engine::{Comment, Thread};

use crate::app::GitViewer;
use super::super::comment::{CommentCtx, Place};
use crate::state::DetailState;
use crate::ui::atoms::{Icon, IconName};
use crate::ui::theme;

fn author(c: &Comment) -> String {
    c.author.name.clone().unwrap_or_else(|| c.author.username.clone())
}

impl GitViewer {
    pub(super) fn render_inline_thread(&self, _fi: usize, ti: usize, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let Some(t) = d.detail.as_ref().and_then(|det| det.threads.get(ti)) else { return div().into_any_element() };
        let inline = &d.code.inline;
        let resolved = t.resolved == Some(true);
        let open = inline.expanded.contains(&t.id);
        let body = match (resolved, open) {
            (true, false) => self.resolved_line(t, false, cx).into_any_element(),
            (true, true) => div().flex().flex_col().gap(theme::SPACE_1P5).child(self.resolved_line(t, true, cx)).child(self.thread_card(t, d, cx)).into_any_element(),
            _ => {
                // Unresolved: a left accent bar, amber when the thread waits for me.
                let bar = match crate::state::attention(d).get(ti) {
                    Some(gv_engine::Attention::NeedsMe) => Some(theme::warning()),
                    Some(gv_engine::Attention::Open) => Some(theme::accent()),
                    _ => None,
                };
                self.thread_card(t, d, cx).when_some(bar, |c, col| c.border_l_2().border_color(col)).into_any_element()
            }
        };
        self.card_side().px(theme::SPACE_3).py(theme::SPACE_2).font_family(theme::ui_font()).child(body).into_any_element()
    }

    /// "Resolved thread by X" header; clicking it (or its chevron) expands or collapses the thread.
    fn resolved_line(&self, t: &Thread, open: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let who = t.comments.first().map(author).unwrap_or_default();
        let tid = t.id.clone();
        div()
            .id(gpui::SharedString::from(format!("resolved-{}", t.id)))
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .px(theme::SPACE_3)
            .py(theme::SPACE_1P5)
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::border_subtle())
            .text_size(theme::text_xs())
            .text_color(theme::text_subtle())
            .cursor_pointer()
            .hover(|s| s.bg(theme::surface_hover()))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    let set = &mut d.code.inline.expanded;
                    if !set.remove(&tid) {
                        set.insert(tid.clone());
                    }
                }
                cx.notify()
            }))
            .child(Icon::new(IconName::Check).size(theme::ICON_SM).color(theme::success()))
            .child(div().flex_1().child(format!("Resolved thread by {who}")))
            .child(format!("{} comment{}", t.comments.len(), if t.comments.len() == 1 { "" } else { "s" }))
            .tooltip(crate::ui::atoms::tooltip(if open { "Collapse thread" } else { "Expand thread" }))
            .child(Icon::new(if open { IconName::ChevronUp } else { IconName::ChevronDown }).size(theme::ICON_SM).color(theme::text_subtle()))
    }

    /// Timeline-style thread: first comment, nested replies, a quiet reply field.
    fn thread_card(&self, t: &Thread, d: &DetailState, cx: &mut Context<Self>) -> Div {
        let inline = &d.code.inline;
        let pending = t.id == "pending";
        let tid = (!pending).then(|| t.id.clone());
        let mut card = div().flex().flex_col().gap(theme::SPACE_3).px(theme::SPACE_3).py(theme::SPACE_3).rounded(theme::RADIUS_MD).bg(theme::surface());
        for (i, c) in t.comments.iter().enumerate() {
            let editing = inline.editing.as_deref() == Some(c.id.as_str());
            let ctx = CommentCtx { place: Place::Code, thread: tid.clone(), resolved: if i == 0 { t.resolved } else { None }, reply: i > 0 };
            let row = if editing { self.inline_field("Save", cx) } else { self.comment_row(c, d, ctx, cx) };
            card = card.child(if i == 0 { row } else { div().pl(theme::SPACE_6 + theme::SPACE_3).child(row) });
        }
        if inline.reply_to.as_deref() == Some(t.id.as_str()) {
            return card.child(div().pl(theme::SPACE_6 + theme::SPACE_3).child(self.inline_field("Reply", cx)));
        }
        let Some(tid) = tid else { return card };
        card.child(
            div()
                .id(gpui::SharedString::from(format!("reply-field-{}", t.id)))
                .ml(theme::SPACE_6 + theme::SPACE_3)
                .px(theme::SPACE_2P5)
                .py(theme::SPACE_1P5)
                .rounded(theme::RADIUS_MD)
                .border_1()
                .border_color(theme::border_subtle())
                .text_size(theme::text_sm())
                .text_color(theme::text_subtle())
                .cursor_text()
                .hover(|s| s.bg(theme::surface_hover()))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.open_inline_field(Some(tid.clone()), None, window, cx)))
                .child("Reply..."),
        )
    }
}
