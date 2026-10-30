//! One comment in the timeline style shared by the Activity tab and Code tab threads:
//! avatar, name and relative time (exact time on hover), markdown body, ghost actions on hover.

use gpui::{div, prelude::*, ClickEvent, Context, Div, Focusable, SharedString};
use gv_engine::{Comment, ProviderKind};

use crate::app::GitViewer;
use crate::features::shared::relative_time;
use crate::state::{Copied, DetailState};
use crate::ui::atoms::{tooltip, Avatar, Button, ButtonVariant, IconName};
use crate::ui::theme;

/// Where the comment is shown; decides where Reply and Edit type.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Place {
    /// Activity tab: the pinned composer.
    Activity,
    /// Code tab: the inline field under the thread.
    Code,
}

/// What the hover actions act on.
pub(super) struct CommentCtx {
    pub place: Place,
    /// Thread to reply to (on the first comment only).
    pub thread: Option<String>,
    /// Thread resolve state, when the thread can be resolved (first comment only).
    pub resolved: Option<bool>,
    /// Nested reply: smaller avatar.
    pub reply: bool,
}

fn display_name(c: &Comment) -> String {
    c.author.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| c.author.username.clone())
}

/// A link to the comment where the provider has a stable anchor, else the PR.
fn comment_link(web_url: &str, kind: ProviderKind, id: &str) -> String {
    match kind {
        ProviderKind::GitLab if id.chars().all(|c| c.is_ascii_digit()) => format!("{web_url}#note_{id}"),
        _ => web_url.to_string(),
    }
}

impl GitViewer {
    pub(super) fn comment_row(&self, c: &Comment, d: &DetailState, ctx: CommentCtx, cx: &mut Context<Self>) -> Div {
        let pending = c.id == "pending";
        let mine = !pending && d.me.as_deref().is_some_and(|m| m.eq_ignore_ascii_case(&c.author.username));
        let group: SharedString = format!("comment-{}", c.id).into();
        let when = if pending { "posting...".to_string() } else { relative_time(c.created_at) };
        let exact = c.created_at.with_timezone(&chrono::Local).format("%b %-d, %Y %H:%M").to_string();
        let size = if ctx.reply { theme::SPACE_5 } else { theme::SPACE_6 };
        let act = |id: String, icon: IconName, tip: &'static str| Button::new(id, "").icon(icon).variant(ButtonVariant::Ghost).tooltip(tip);
        let mut actions = div().flex().items_center().gap(theme::SPACE_0P5).opacity(0.).group_hover(group.clone(), |s| s.opacity(1.));
        if !pending {
            if let Some(tid) = ctx.thread.clone() {
                let place = ctx.place;
                actions = actions.child(act(format!("reply-{}", c.id), IconName::CornerUpLeft, "Reply").on_click(cx.listener(move |this, _: &ClickEvent, window, cx| match place {
                    Place::Activity => {
                        if let Some(d) = this.detail.as_mut() {
                            d.reply_to = Some(tid.clone());
                            d.editing = None;
                        }
                        window.focus(&this.composer.focus_handle(cx));
                        cx.notify()
                    }
                    Place::Code => this.open_inline_field(Some(tid.clone()), None, window, cx),
                })));
            }
            if mine {
                let (cid, body, place) = (c.id.clone(), c.body.clone(), ctx.place);
                actions = actions.child(act(format!("edit-{}", c.id), IconName::Pencil, "Edit").on_click(cx.listener(move |this, _: &ClickEvent, window, cx| match place {
                    Place::Activity => {
                        if let Some(d) = this.detail.as_mut() {
                            d.editing = Some(cid.clone());
                            d.reply_to = None;
                        }
                        this.composer.update(cx, |i, cx| i.set_text(body.clone(), cx));
                        window.focus(&this.composer.focus_handle(cx));
                        cx.notify()
                    }
                    Place::Code => this.open_inline_field(None, Some((cid.clone(), body.clone())), window, cx),
                })));
            }
            let link = comment_link(&d.summary.web_url, d.summary.id.repo.kind, &c.id);
            actions = actions.child(act(format!("link-{}", c.id), IconName::Link, "Copy link").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.copy(Copied::Link, link.clone(), cx))));
            if let (Some(res), Some(tid)) = (ctx.resolved, ctx.thread.clone()) {
                let (icon, tip) = if res { (IconName::ArrowBackUp, "Unresolve") } else { (IconName::Check, "Resolve") };
                actions = actions.child(act(format!("resolve-{}", c.id), icon, tip).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_thread_resolved(tid.clone(), !res, cx))));
            }
        }
        let head = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .min_h(theme::SPACE_6)
            .text_size(theme::text_sm())
            .child(div().font_weight(theme::WEIGHT_MEDIUM).text_color(theme::text()).child(display_name(c)))
            .child(div().id(SharedString::from(format!("when-{}", c.id))).text_color(theme::text_subtle()).tooltip(tooltip(exact)).child(when))
            .child(div().flex_1())
            .child(actions);
        // GitLab notes carry no award emoji (one request per note); Azure DevOps comments have likes.
        let can_react = !pending && matches!(d.summary.id.repo.kind, ProviderKind::GitHub | ProviderKind::Azure);
        let reactions = self.reactions_bar(Some(&c.id), &c.reactions, group.clone(), can_react, cx);
        div()
            .group(group)
            .flex()
            .gap(theme::SPACE_3)
            .child(Avatar::new(c.author.username.clone()).size(size).image(self.avatar(c.author.avatar_url.as_deref())))
            .child(div().flex().flex_col().flex_1().min_w(theme::SPACE_0).gap(theme::SPACE_1).child(head).child(self.markdown(format!("c-{}", c.id), &c.body)).children(reactions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_gitlab_notes_only() {
        assert_eq!(comment_link("https://gl/x/-/merge_requests/1", ProviderKind::GitLab, "42"), "https://gl/x/-/merge_requests/1#note_42");
        assert_eq!(comment_link("https://github.com/a/b/pull/1", ProviderKind::GitHub, "IC_x"), "https://github.com/a/b/pull/1");
    }
}
