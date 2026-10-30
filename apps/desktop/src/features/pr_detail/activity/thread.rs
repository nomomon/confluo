//! One conversation or inline thread in the timeline: file:line chip, first comment, nested replies.
//! Unresolved threads carry a left accent bar (amber when they need me) and pills; resolved threads
//! collapse to one muted line with a check (click to expand); outdated ones are muted.

use gpui::{canvas, div, point, prelude::*, px, ClickEvent, Context, SharedString};
use gv_engine::{Attention, Thread};

use super::super::comment::{CommentCtx, Place};
use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::{Badge, BadgeVariant, Button, ButtonVariant, Icon, IconName};
use crate::ui::theme;

/// Indent of thread content past the avatar column.
fn indent() -> gpui::Pixels {
    theme::SPACE_6 + theme::SPACE_3
}

impl GitViewer {
    pub(super) fn render_thread(&self, t: &Thread, a: Attention, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let tid = (t.id != "pending").then(|| t.id.clone());
        let expanded = d.code.inline.expanded.contains(&t.id);
        let collapsed = a == Attention::Resolved && !expanded;
        let mut col = div().flex().flex_col().gap(theme::SPACE_3);
        if a == Attention::Resolved {
            col = col.child(self.resolved_summary(t, expanded, cx));
        }
        if !collapsed {
            col = self.thread_body(col, t, a, tid, d, cx);
        }
        let bar = match a {
            Attention::NeedsMe => Some(theme::warning()),
            Attention::Open => Some(theme::accent()),
            _ => None,
        };
        // "Next unresolved" scrolls the Conversation to this thread on its next paint.
        let (reveal, handle, id) = (self.review.reveal_conv.clone(), self.review.right_scroll.clone(), t.id.clone());
        let marker = canvas(
            move |b, window, _| {
                if reveal.borrow().as_deref() == Some(id.as_str()) {
                    reveal.borrow_mut().take();
                    let view = handle.bounds();
                    let off = handle.offset();
                    handle.set_offset(point(off.x, off.y - (b.origin.y - view.origin.y) + theme::SPACE_3));
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let current = self.review.attn_at.as_deref() == Some(t.id.as_str());
        div()
            .relative()
            .when(matches!(a, Attention::Outdated), |e| e.opacity(0.65))
            .when(current, |e| e.child(div().absolute().top(-theme::SPACE_1).bottom(-theme::SPACE_1).left(-theme::SPACE_2).right(-theme::SPACE_1).rounded(theme::RADIUS_MD).bg(theme::surface())))
            .children(bar.map(|c| div().absolute().top_0().bottom_0().left(-theme::SPACE_2).w(px(2.)).rounded(theme::RADIUS_FULL).bg(c)))
            .child(marker)
            .child(col)
    }

    fn thread_body(&self, mut col: gpui::Div, t: &Thread, a: Attention, tid: Option<String>, d: &DetailState, cx: &mut Context<Self>) -> gpui::Div {
        let location = t.position.as_ref().map(|p| {
            let range = p.start_line.map(|s| format!("{s}-")).unwrap_or_default();
            format!("{}:{range}{}", p.path, p.line)
        });
        let location = location.or_else(|| t.comments.first().and_then(|c| c.position.as_ref()).map(|p| format!("{}{}", p.path, p.line.map(|l| format!(":{l}")).unwrap_or_default())));
        let pills = matches!(a, Attention::NeedsMe | Attention::Open | Attention::Outdated);
        if location.is_some() || pills {
            let show = tid.clone().filter(|_| t.position.is_some());
            col = col.child(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_2)
                    .pl(indent())
                    .children(location.map(|loc| div().min_w(theme::SPACE_0).truncate().px(theme::SPACE_2).py(theme::SPACE_0P5).rounded(theme::RADIUS_SM).bg(theme::surface()).text_size(theme::text_xs()).font_family(theme::mono_font()).text_color(theme::text_muted()).child(loc)))
                    .when(a == Attention::NeedsMe, |d| d.child(Badge::new("Needs you").tint(theme::warning())))
                    .when(matches!(a, Attention::NeedsMe | Attention::Open), |d| d.child(Badge::new("Unresolved").variant(BadgeVariant::Outline)))
                    .when(a == Attention::Outdated, |d| d.child(Badge::new("Outdated")))
                    .children(show.map(|id| {
                        Button::new(format!("show-in-code-{id}"), "Show in code").icon(IconName::Code).variant(ButtonVariant::Ghost).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_thread_in_code(id.clone(), cx)))
                    })),
            );
        }
        for (i, c) in t.comments.iter().enumerate() {
            let ctx = CommentCtx { place: Place::Activity, thread: tid.clone(), resolved: if i == 0 { t.resolved } else { None }, reply: i > 0, inline: t.position.is_some() };
            let row = self.comment_row(c, d, ctx, cx);
            col = col.child(if i == 0 { row } else { div().pl(indent()).child(row) });
        }
        col
    }

    /// One muted line for a resolved thread: check, who, where, count; click expands or collapses it.
    fn resolved_summary(&self, t: &Thread, open: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let who = t.comments.first().map(|c| c.author.name.clone().unwrap_or_else(|| c.author.username.clone())).unwrap_or_default();
        let place = t.position.as_ref().map(|p| format!(" on {}", p.path.rsplit('/').next().unwrap_or(&p.path))).unwrap_or_default();
        let n = t.comments.len();
        let tid = t.id.clone();
        div()
            .id(SharedString::from(format!("conv-resolved-{}", t.id)))
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .pl(indent())
            .pr(theme::SPACE_1)
            .py(theme::SPACE_1)
            .rounded(theme::RADIUS_SM)
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
            .child(div().flex_1().min_w(theme::SPACE_0).truncate().child(format!("Resolved thread by {who}{place}")))
            .child(format!("{n} comment{}", if n == 1 { "" } else { "s" }))
            .child(Icon::new(if open { IconName::ChevronUp } else { IconName::ChevronDown }).size(theme::ICON_SM).color(theme::text_subtle()))
    }
}
