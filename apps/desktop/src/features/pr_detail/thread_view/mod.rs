//! One review thread, GitLab style, shared by the Conversation and the diff: a flat container (hairline
//! border, small radius, neutral fill, no colored edge). The Conversation adds a header (path, copy,
//! status text, Show in code) and a few lines of code context; then the comments split by hairlines
//! (replies fold behind "Show N replies" past two), and a footer with Reply and Resolve / Reopen thread.
//! Attention is text only: "Needs you" in amber, "Unresolved" muted.

mod body;
mod footer;
mod header;
mod suggestion;

use gpui::{div, prelude::*, ClickEvent, Context, Div, SharedString};
use gv_engine::{Attention, Comment, Thread};

use super::comment::Place;
use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::{Icon, IconName};
use crate::ui::theme;

pub(super) fn display_name(c: &Comment) -> String {
    c.author
        .name
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| c.author.username.clone())
}

/// Hairline between thread sections.
pub(super) fn section() -> Div {
    div().border_t_1().border_color(theme::border_subtle())
}

impl GitViewer {
    pub(super) fn thread_view(
        &self,
        t: &Thread,
        a: Attention,
        d: &DetailState,
        place: Place,
        cx: &mut Context<Self>,
    ) -> Div {
        let resolved = t.resolved == Some(true);
        let collapsed = resolved && !d.code.inline.expanded.contains(&t.id);
        let mut c = div()
            .flex()
            .flex_col()
            .w_full()
            .rounded(theme::RADIUS_SM)
            .border_1()
            .border_color(theme::border())
            .bg(theme::container_bg())
            .when(a == Attention::Outdated, |e| e.opacity(0.7));
        let mut first = true;
        if place == Place::Activity {
            if let Some(h) = self.thread_header(t, a, cx) {
                c = c.child(h);
                first = false;
                if !collapsed {
                    c = c.children(self.thread_snippet(t, d));
                }
            }
        }
        if resolved {
            let bar = self.resolved_bar(t, !collapsed, cx);
            c = c.child(if first {
                div().child(bar)
            } else {
                section().child(bar)
            });
            first = false;
            if collapsed {
                return c;
            }
        }
        c = self.thread_comments(c, t, d, place, first, cx);
        c.children(self.thread_footer(t, d, place, cx))
    }

    /// One muted line: check, who resolved it, comment count, chevron; click expands or collapses.
    fn resolved_bar(&self, t: &Thread, open: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let who = t
            .resolved_by
            .as_ref()
            .map(|u| u.name.clone().unwrap_or_else(|| u.username.clone()))
            .or_else(|| t.comments.first().map(display_name))
            .unwrap_or_default();
        let n = t.comments.len();
        let tid = t.id.clone();
        div()
            .id(SharedString::from(format!("thread-resolved-{}", t.id)))
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .px(theme::SPACE_3)
            .py(theme::SPACE_2)
            .rounded(theme::RADIUS_SM)
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
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
            .child(
                Icon::new(IconName::CircleCheckFilled)
                    .size(theme::ICON_MD)
                    .color(theme::success()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(theme::SPACE_0)
                    .truncate()
                    .child(format!("Thread resolved by {who}")),
            )
            .child(format!("{n} comment{}", if n == 1 { "" } else { "s" }))
            .child(
                Icon::new(if open {
                    IconName::ChevronUp
                } else {
                    IconName::ChevronDown
                })
                .size(theme::ICON_SM)
                .color(theme::text_subtle()),
            )
    }
}
