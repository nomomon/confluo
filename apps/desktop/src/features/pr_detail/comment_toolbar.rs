//! Hover toolbar of a comment (Slack style): recent reactions, add reaction, Reply, Resolve and More,
//! anchored at the row's top right, raised over the previous row so it never covers the author line. More lists only what the toolbar lacks.

use gpui::{div, prelude::*, ClickEvent, Context, SharedString};
use gv_engine::{ProviderKind, ReactionKind};

use super::comment_actions::TOOLBAR;
use super::comment_menu::EMOJI_FONT;
use super::comment_run::CommentTarget;
use crate::app::{GitViewer, Layer};
use crate::ui::atoms::{Button, ButtonVariant, IconName};
use crate::ui::theme;

/// Toolbar emoji: the three most recent reactions the provider supports.
fn toolbar_reactions(mru: &[ReactionKind], kind: ProviderKind) -> Vec<ReactionKind> {
    let ok = ReactionKind::for_provider(kind);
    let mut out: Vec<ReactionKind> = mru.iter().copied().filter(|k| ok.contains(k)).collect();
    for k in [
        ReactionKind::ThumbsUp,
        ReactionKind::Heart,
        ReactionKind::Hooray,
    ] {
        if ok.contains(&k) && !out.contains(&k) {
            out.push(k);
        }
    }
    out.truncate(3);
    out
}

impl GitViewer {
    /// Floating toolbar at the top right, shown on hover (and while its own menu or picker is open);
    /// hidden while any other layer is open.
    pub(super) fn comment_toolbar(
        &self,
        t: &CommentTarget,
        group: SharedString,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let mine_open = matches!(&self.layer, Some(Layer::CommentMenu(id)) | Some(Layer::Reactions(id)) if *id == t.id)
            || crate::dev::hover_comment().is_some_and(|h| h == t.id);
        if self.layer_open() && !mine_open {
            // A right-click menu of this comment still renders (at the pointer).
            return self.comment_menu(t, cx);
        }
        let act = |id: String, icon: IconName, tip: &'static str| {
            Button::new(id, "")
                .icon(icon)
                .variant(ButtonVariant::Ghost)
                .tooltip(tip)
        };
        let kind = self
            .detail
            .as_ref()
            .map(|d| d.summary.id.repo.kind)
            .unwrap_or(ProviderKind::GitHub);
        let mut bar = div()
            .flex()
            .items_center()
            .gap(theme::space_0p5())
            .p(theme::space_0p5())
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_md();
        if t.can_react {
            for k in toolbar_reactions(&self.settings.reaction_mru, kind) {
                let id = t.id.clone();
                bar = bar.child(
                    div()
                        .id(SharedString::from(format!("tb-rx-{}-{k:?}", t.id)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(theme::space_7())
                        .rounded(theme::RADIUS_SM)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme::surface_hover()))
                        .font_family(EMOJI_FONT)
                        .text_size(theme::text_sm())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.toggle_reaction(Some(id.clone()), k, cx)
                        }))
                        .child(k.emoji()),
                );
            }
            bar = bar.child(self.add_reaction_button(&t.id, cx));
        }
        for a in TOOLBAR {
            if a.available(&t.facts) {
                let t2 = t.clone();
                bar = bar.child(
                    act(format!("tb-{a:?}-{}", t.id), a.icon(), a.label()).on_click(cx.listener(
                        move |this, _: &ClickEvent, window, cx| {
                            this.run_comment_action(a, &t2, window, cx)
                        },
                    )),
                );
            }
        }
        let toggle = t.id.clone();
        let more = act(format!("tb-more-{}", t.id), IconName::Ellipsis, "More").on_click(
            cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.toggle_layer(Layer::CommentMenu(toggle.clone()), cx)
            }),
        );
        bar = bar.child(
            div()
                .relative()
                .child(more)
                .children(self.comment_menu(t, cx)),
        );
        Some(
            div()
                .absolute()
                .top(-theme::space_7())
                .right(theme::space_2())
                .opacity(if mine_open { 1. } else { 0. })
                .group_hover(group, |s| s.opacity(1.))
                .child(bar)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_shows_three_recent_supported_reactions() {
        let mru = [ReactionKind::Rocket, ReactionKind::Eyes];
        assert_eq!(
            toolbar_reactions(&mru, ProviderKind::GitHub),
            vec![
                ReactionKind::Rocket,
                ReactionKind::Eyes,
                ReactionKind::ThumbsUp
            ]
        );
        assert_eq!(
            toolbar_reactions(&[], ProviderKind::GitHub),
            vec![
                ReactionKind::ThumbsUp,
                ReactionKind::Heart,
                ReactionKind::Hooray
            ]
        );
        // Azure comments only have a like.
        assert_eq!(
            toolbar_reactions(&mru, ProviderKind::Azure),
            vec![ReactionKind::ThumbsUp]
        );
    }
}
