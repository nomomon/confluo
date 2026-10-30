//! One comment in the timeline style shared by the Conversation and Code tab threads: avatar, name and
//! relative time (exact time on hover), markdown body, reaction chips. Hovering shows a floating toolbar at
//! the top right (Slack style, no layout shift): the three most recent reactions, add reaction, Reply,
//! Resolve and More; right-click opens the same menu at the pointer. Actions come from `comment_actions`.

use gpui::{div, prelude::*, ClickEvent, Context, Div, MouseButton, MouseDownEvent, SharedString};
use gv_engine::{Comment, ProviderKind, ReactionKind};

use super::comment_actions::{comment_link, CommentAction, CommentFacts};
use super::comment_menu::{CommentTarget, EMOJI_FONT};
use crate::app::{GitViewer, Layer};
use crate::features::shared::relative_time;
use crate::state::DetailState;
use crate::ui::atoms::{tooltip, Avatar, Button, ButtonVariant, IconName};
use crate::ui::theme;

/// Where the comment is shown; decides where Reply and Edit type.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Place {
    /// Conversation tab: the pinned composer.
    Activity,
    /// Code tab: the inline field under the thread.
    Code,
}

/// What the actions act on.
pub(super) struct CommentCtx {
    pub place: Place,
    /// Thread to reply to.
    pub thread: Option<String>,
    /// Thread resolve state, when the thread can be resolved (first comment only).
    pub resolved: Option<bool>,
    /// Nested reply: smaller avatar.
    pub reply: bool,
    /// The thread sits on a diff line.
    pub inline: bool,
}

fn display_name(c: &Comment) -> String {
    c.author.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| c.author.username.clone())
}

/// Toolbar emoji: the three most recent reactions the provider supports.
fn toolbar_reactions(mru: &[ReactionKind], kind: ProviderKind) -> Vec<ReactionKind> {
    let ok = ReactionKind::for_provider(kind);
    let mut out: Vec<ReactionKind> = mru.iter().copied().filter(|k| ok.contains(k)).collect();
    for k in [ReactionKind::ThumbsUp, ReactionKind::Heart, ReactionKind::Hooray] {
        if ok.contains(&k) && !out.contains(&k) {
            out.push(k);
        }
    }
    out.truncate(3);
    out
}

impl GitViewer {
    pub(super) fn comment_row(&self, c: &Comment, d: &DetailState, ctx: CommentCtx, cx: &mut Context<Self>) -> Div {
        let pending = c.id == "pending";
        let mine = !pending && d.me.as_deref().is_some_and(|m| m.eq_ignore_ascii_case(&c.author.username));
        let group: SharedString = format!("comment-{}", c.id).into();
        let when = if pending { "posting...".to_string() } else { relative_time(c.created_at) };
        let exact = c.created_at.with_timezone(&chrono::Local).format("%b %-d, %Y %H:%M").to_string();
        let size = if ctx.reply { theme::SPACE_5 } else { theme::SPACE_6 };
        let kind = d.summary.id.repo.kind;
        // GitLab notes: award emoji arrive lazily for notes on screen; reacting waits until they are known.
        d.want_awards(&c.id);
        let can_react = !pending && (matches!(kind, ProviderKind::GitHub | ProviderKind::Azure) || (kind == ProviderKind::GitLab && !d.awards_unknown(&c.id)));
        let facts = CommentFacts { mine, pending, can_reply: ctx.thread.is_some(), resolved: ctx.resolved, inline: ctx.inline, in_conversation: ctx.place == Place::Activity };
        let t = CommentTarget { id: c.id.clone(), body: c.body.clone(), thread: ctx.thread.clone(), place: ctx.place, link: comment_link(&d.summary.web_url, kind, &c.id), facts, can_react };
        let head = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .min_h(theme::SPACE_6)
            .text_size(theme::text_sm())
            .child(div().font_weight(theme::WEIGHT_MEDIUM).text_color(theme::text()).child(display_name(c)))
            .child(div().id(SharedString::from(format!("when-{}", c.id))).text_color(theme::text_subtle()).tooltip(tooltip(exact)).child(when));
        let reactions = self.reactions_bar(Some(&c.id), &c.reactions, group.clone(), can_react, cx);
        let rt = t.id.clone();
        div()
            .group(group.clone())
            .relative()
            .flex()
            .gap(theme::SPACE_3)
            .when(!pending, |e| {
                e.on_mouse_down(MouseButton::Right, cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    this.layer = Some(Layer::CommentMenuAt(rt.clone(), ev.position));
                    cx.stop_propagation();
                    cx.notify()
                }))
            })
            .child(Avatar::new(c.author.username.clone()).size(size).image(self.avatar(c.author.avatar_url.as_deref())))
            .child(div().flex().flex_col().flex_1().min_w(theme::SPACE_0).gap(theme::SPACE_1).child(head).child(self.markdown(format!("c-{}", c.id), &c.body)).children(reactions))
            .children((!pending).then(|| self.comment_toolbar(&t, group, cx)).flatten())
    }

    /// Floating toolbar at the top right, shown on hover (and while its own menu or picker is open);
    /// hidden while any other layer is open.
    fn comment_toolbar(&self, t: &CommentTarget, group: SharedString, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let mine_open = matches!(&self.layer, Some(Layer::CommentMenu(id)) | Some(Layer::Reactions(id)) if *id == t.id)
            || crate::dev::hover_comment().is_some_and(|h| h == t.id);
        if self.layer_open() && !mine_open {
            // A right-click menu of this comment still renders (at the pointer).
            return self.comment_menu(t, cx);
        }
        let act = |id: String, icon: IconName, tip: &'static str| Button::new(id, "").icon(icon).variant(ButtonVariant::Ghost).tooltip(tip);
        let kind = self.detail.as_ref().map(|d| d.summary.id.repo.kind).unwrap_or(ProviderKind::GitHub);
        let mut bar = div().flex().items_center().gap(theme::SPACE_0P5).p(theme::SPACE_0P5).rounded(theme::RADIUS_MD).border_1().border_color(theme::border()).bg(theme::overlay()).shadow_md();
        if t.can_react {
            for k in toolbar_reactions(&self.settings.reaction_mru, kind) {
                let id = t.id.clone();
                bar = bar.child(
                    div()
                        .id(SharedString::from(format!("tb-rx-{}-{k:?}", t.id)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(theme::control_h())
                        .rounded(theme::RADIUS_SM)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme::surface_hover()))
                        .font_family(EMOJI_FONT)
                        .text_size(theme::text_sm())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_reaction(Some(id.clone()), k, cx)))
                        .child(k.emoji()),
                );
            }
            bar = bar.child(self.add_reaction_button(&t.id, cx));
        }
        for a in [CommentAction::Reply, CommentAction::Resolve, CommentAction::Unresolve] {
            if a.available(&t.facts) {
                let t2 = t.clone();
                bar = bar.child(act(format!("tb-{a:?}-{}", t.id), a.icon(), a.label()).on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run_comment_action(a, &t2, window, cx))));
            }
        }
        let toggle = t.id.clone();
        let more = act(format!("tb-more-{}", t.id), IconName::Ellipsis, "More").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_layer(Layer::CommentMenu(toggle.clone()), cx)));
        bar = bar.child(div().relative().child(more).children(self.comment_menu(t, cx)));
        Some(div().absolute().top(-theme::SPACE_4).right_0().opacity(if mine_open { 1. } else { 0. }).group_hover(group, |s| s.opacity(1.)).child(bar).into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_shows_three_recent_supported_reactions() {
        let mru = [ReactionKind::Rocket, ReactionKind::Eyes];
        assert_eq!(toolbar_reactions(&mru, ProviderKind::GitHub), vec![ReactionKind::Rocket, ReactionKind::Eyes, ReactionKind::ThumbsUp]);
        assert_eq!(toolbar_reactions(&[], ProviderKind::GitHub), vec![ReactionKind::ThumbsUp, ReactionKind::Heart, ReactionKind::Hooray]);
        // Azure comments only have a like.
        assert_eq!(toolbar_reactions(&mru, ProviderKind::Azure), vec![ReactionKind::ThumbsUp]);
    }
}
