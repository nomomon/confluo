//! One comment row of a thread (GitLab layout): avatar, name, @handle, relative time (exact on hover),
//! neutral role pills (Author, Bot), the resolved check, markdown body with suggestions, reaction chips.
//! Hovering highlights the row and shows a floating toolbar at its top right, raised over the previous row (Slack style,
//! no layout shift): recent reactions, add reaction, Reply, Resolve and More (only what the toolbar
//! lacks); right-click opens the full menu at the pointer. Actions come from `comment_actions`.

use gpui::{div, prelude::*, Context, Div, MouseButton, MouseDownEvent, SharedString, Stateful};
use gv_engine::{Comment, DiffPosition, ProviderKind};

use super::comment_actions::CommentFacts;
use super::comment_links::comment_link;
use super::comment_run::CommentTarget;
use super::thread_view::display_name;
use crate::app::{GitViewer, Layer};
use crate::features::shared::relative_time;
use crate::state::{CommentKey, DetailState};
use crate::ui::atoms::{tooltip, Avatar, Badge, BadgeVariant, Icon, IconName};
use crate::ui::theme;

/// Where the comment is shown; decides where Reply and Edit type and keys its transient UI.
pub(super) use crate::state::Surface as Place;

/// What the actions act on.
pub(super) struct CommentCtx {
    pub place: Place,
    /// Thread to reply to.
    pub thread: Option<String>,
    /// Thread resolve state, when the thread can be resolved (first comment only).
    pub resolved: Option<bool>,
    /// The thread sits on a diff line.
    pub inline: bool,
    /// The thread's anchor (suggestions read the original lines there).
    pub pos: Option<DiffPosition>,
    /// "Resolved 2h ago by Ada" under the first comment of a resolved thread.
    pub resolved_line: Option<String>,
}

/// Neutral role pills: Author (wrote the PR) and Bot.
pub(super) fn roles(c: &Comment, pr_author: &str) -> Vec<&'static str> {
    let mut out = vec![];
    if c.author.username.eq_ignore_ascii_case(pr_author) {
        out.push("Author");
    }
    if c.author.bot {
        out.push("Bot");
    }
    out
}

/// Own comment: Edit and Delete are offered only then. Decided per rendered comment (the anchor).
pub(super) fn is_mine(c: &Comment, me: Option<&str>) -> bool {
    me.is_some_and(|m| m.eq_ignore_ascii_case(&c.author.username))
}

impl GitViewer {
    pub(super) fn comment_row(
        &self,
        c: &Comment,
        d: &DetailState,
        ctx: CommentCtx,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let queued = self
            .outbox
            .get(&d.summary.id)
            .and_then(|o| o.pending_for(&c.id))
            .cloned();
        let pending = c.id.starts_with(crate::state::outbox::PREFIX) || queued.is_some();
        let sending = queued
            .as_ref()
            .is_some_and(|p| p.status == crate::state::outbox::Status::Sending);
        let mine = !pending && is_mine(c, d.me.as_deref());
        let key = CommentKey::new(ctx.place, c.id.clone());
        let group: SharedString = format!("comment-{}", key.tag()).into();
        let when = relative_time(c.created_at);
        let exact = c
            .created_at
            .with_timezone(&chrono::Local)
            .format("%b %-d, %Y %H:%M")
            .to_string();
        let kind = d.summary.id.repo.kind;
        // GitLab notes: award emoji arrive lazily for notes on screen; reacting waits until they are known.
        d.want_awards(&c.id);
        let can_react = !pending
            && (matches!(kind, ProviderKind::GitHub | ProviderKind::Azure)
                || (kind == ProviderKind::GitLab && !d.awards_unknown(&c.id)));
        let facts = CommentFacts {
            mine,
            pending,
            can_reply: ctx.thread.is_some(),
            resolved: ctx.resolved,
            inline: ctx.inline,
            in_conversation: ctx.place == Place::Activity,
        };
        let t = CommentTarget {
            key: key.clone(),
            id: c.id.clone(),
            body: c.body.clone(),
            thread: ctx.thread.clone(),
            place: ctx.place,
            link: comment_link(&d.summary.web_url, kind, &c.id),
            facts,
            can_react,
        };
        let name = display_name(c);
        let handle = (name != c.author.username).then(|| format!("@{}", c.author.username));
        let head = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(theme::space_1p5())
            .min_h(theme::space_7())
            .text_size(theme::text_sm())
            .child(
                div()
                    .font_weight(theme::WEIGHT_MEDIUM)
                    .text_color(theme::text())
                    .child(name),
            )
            .children(handle.map(|h| div().text_color(theme::text_subtle()).child(h)))
            .child(match &queued {
                Some(p) => self.pending_meta(p, cx).into_any_element(),
                None => div()
                    .id(SharedString::from(format!("when-{}", c.id)))
                    .text_color(theme::text_subtle())
                    .tooltip(tooltip(exact))
                    .child(when)
                    .into_any_element(),
            })
            .children(
                roles(c, &d.summary.author.username)
                    .into_iter()
                    .map(|r| Badge::new(r).variant(BadgeVariant::Outline)),
            )
            .when(ctx.resolved == Some(true), |e| {
                e.child(
                    div()
                        .id(SharedString::from(format!("resolved-mark-{}", c.id)))
                        .tooltip(tooltip("Thread resolved"))
                        .child(
                            Icon::new(IconName::CircleCheckFilled)
                                .size(theme::icon_lg())
                                .color(theme::success()),
                        ),
                )
            });
        let resolved_line = ctx.resolved_line.clone().map(|l| {
            div()
                .text_size(theme::text_xs())
                .text_color(theme::text_subtle())
                .child(l)
        });
        let body = self.comment_body(c, ctx.pos.as_ref(), d, cx);
        let reactions = self.reactions_bar(Some(&c.id), &c.reactions, group.clone(), can_react, cx);
        let rt = t.key.clone();
        let hk = key.clone();
        div()
            .id(SharedString::from(format!("row-{}", key.tag())))
            .group(group.clone())
            .relative()
            .flex()
            .gap(theme::space_3())
            .px(theme::space_3())
            .py(theme::space_2p5())
            .rounded(theme::RADIUS_SM)
            .hover(|s| s.bg(theme::surface_hover()))
            .when(sending, |e| e.opacity(theme::OPACITY_PENDING))
            .on_hover(cx.listener(move |this, h: &bool, _, cx| this.hover_comment_row(&hk, *h, cx)))
            .when(!pending, |e| {
                e.on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                        this.layer = Some(Layer::CommentMenuAt(rt.clone(), ev.position));
                        cx.stop_propagation();
                        cx.notify()
                    }),
                )
            })
            .child(
                Avatar::new(c.author.username.clone())
                    .size(theme::space_7())
                    .image(self.avatar(c.author.avatar_url.as_deref())),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(theme::space_0())
                    .gap(theme::space_1())
                    .child(head)
                    .children(resolved_line)
                    .child(body)
                    .children(reactions),
            )
            .children((!pending).then(|| self.comment_toolbar(&t, cx)).flatten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_author_and_bot() {
        let mut c = Comment {
            id: "1".into(),
            author: gv_engine::User {
                username: "Ada".into(),
                name: None,
                avatar_url: None,
                bot: true,
                id: None,
            },
            body: String::new(),
            created_at: chrono::Utc::now(),
            position: None,
            reactions: vec![],
            suggestions: vec![],
        };
        assert_eq!(roles(&c, "ada"), vec!["Author", "Bot"]);
        c.author.bot = false;
        assert!(roles(&c, "grace").is_empty());
    }

    #[test]
    fn edit_and_delete_follow_the_anchored_comments_author() {
        use super::super::comment_actions::{CommentAction, CommentFacts};
        let mut bot = Comment {
            id: "9".into(),
            author: gv_engine::User::named("review-bot"),
            body: String::new(),
            created_at: chrono::Utc::now(),
            position: None,
            reactions: vec![],
            suggestions: vec![],
        };
        bot.author.bot = true;
        let facts = |c: &Comment| CommentFacts {
            mine: is_mine(c, Some("me")),
            pending: false,
            can_reply: true,
            resolved: None,
            inline: true,
            in_conversation: true,
        };
        let has = |f: &CommentFacts, a| CommentAction::menu(f).iter().flatten().any(|x| *x == a);
        assert!(!has(&facts(&bot), CommentAction::Edit));
        assert!(!has(&facts(&bot), CommentAction::Delete));
        let mut own = bot.clone();
        own.author = gv_engine::User::named("Me");
        assert!(has(&facts(&own), CommentAction::Edit));
        assert!(has(&facts(&own), CommentAction::Delete));
        assert!(!is_mine(&own, None));
    }
}
