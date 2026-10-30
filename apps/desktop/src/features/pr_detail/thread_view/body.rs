//! The comments of a thread: the first one, then replies split by hairlines. Past two replies a
//! "Collapse replies" / "Show N replies" toggle sits under the first comment.

use gpui::{div, prelude::*, ClickEvent, Context, Div, SharedString};
use gv_engine::Thread;

use super::super::comment::{CommentCtx, Place};
use super::{display_name, section};
use crate::app::GitViewer;
use crate::features::shared::relative_time;
use crate::state::DetailState;
use crate::ui::atoms::{Icon, IconName};
use crate::ui::theme;

/// Replies fold behind a toggle only past this many.
const FOLD_AFTER: usize = 2;

/// Whether the toggle shows, and whether replies are hidden.
pub(super) fn fold_state(replies: usize, folded: bool) -> (bool, bool) {
    let toggle = replies > FOLD_AFTER;
    (toggle, toggle && folded)
}

impl GitViewer {
    pub(super) fn thread_comments(
        &self,
        mut c: Div,
        t: &Thread,
        d: &DetailState,
        place: Place,
        first: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let inline = &d.code.inline;
        let tid = (t.id != "pending").then(|| t.id.clone());
        let replies = t.comments.len().saturating_sub(1);
        let (toggle, hidden) = fold_state(replies, inline.replies_folded.contains(&t.id));
        let resolved_line = (t.resolved == Some(true)).then(|| {
            let who = t
                .resolved_by
                .as_ref()
                .map(|u| u.name.clone().unwrap_or_else(|| u.username.clone()));
            match (t.resolved_at, who) {
                (Some(at), Some(w)) => format!("Resolved {} by {w}", relative_time(at)),
                (None, Some(w)) => format!("Resolved by {w}"),
                _ => "Resolved".to_string(),
            }
        });
        for (i, cm) in t.comments.iter().enumerate() {
            if i > 0 && hidden {
                break;
            }
            if i == 1 && toggle {
                c = c.child(section().child(self.replies_toggle(t, replies, hidden, cx)));
            }
            let row = if place == Place::Code && inline.editing.as_deref() == Some(cm.id.as_str()) {
                div()
                    .px(theme::SPACE_3)
                    .py(theme::SPACE_2P5)
                    .child(self.inline_field("Save", cx))
            } else {
                let ctx = CommentCtx {
                    place,
                    thread: tid.clone(),
                    resolved: if i == 0 { t.resolved } else { None },
                    inline: t.position.is_some(),
                    pos: t.position.clone(),
                    resolved_line: if i == 0 { resolved_line.clone() } else { None },
                };
                self.comment_row(cm, d, ctx, cx)
            };
            c = c.child(if i == 0 && first {
                div().child(row)
            } else {
                section().child(row)
            });
        }
        if hidden {
            if let Some(last) = t.comments.last() {
                c = c.child(
                    section()
                        .px(theme::SPACE_3)
                        .py(theme::SPACE_2)
                        .text_size(theme::text_xs())
                        .text_color(theme::text_subtle())
                        .child(format!(
                            "Last reply by {} {}",
                            display_name(last),
                            relative_time(last.created_at)
                        )),
                );
            }
        }
        c
    }

    fn replies_toggle(
        &self,
        t: &Thread,
        n: usize,
        hidden: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tid = t.id.clone();
        div()
            .id(SharedString::from(format!("replies-toggle-{}", t.id)))
            .flex()
            .items_center()
            .gap(theme::SPACE_1P5)
            .px(theme::SPACE_3)
            .py(theme::SPACE_2)
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
            .cursor_pointer()
            .hover(|s| s.text_color(theme::text()))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    let set = &mut d.code.inline.replies_folded;
                    if !set.remove(&tid) {
                        set.insert(tid.clone());
                    }
                }
                cx.notify()
            }))
            .child(
                Icon::new(if hidden {
                    IconName::ChevronRight
                } else {
                    IconName::ChevronDown
                })
                .size(theme::ICON_SM),
            )
            .child(if hidden {
                format!("Show {n} replies")
            } else {
                "Collapse replies".to_string()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::fold_state;

    #[test]
    fn folds_only_past_two_replies() {
        assert_eq!(fold_state(2, true), (false, false));
        assert_eq!(fold_state(3, false), (true, false));
        assert_eq!(fold_state(3, true), (true, true));
    }
}
