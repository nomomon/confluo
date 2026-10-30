//! Reaction chips under a comment or the description (emoji and count, highlighted when mine,
//! who reacted on hover) and an "add reaction" button with a picker of the eight shared reactions.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, MouseDownEvent, SharedString};
use gv_engine::{ReactionGroup, ReactionKind};

use crate::app::{GitViewer, Layer};
use crate::ui::atoms::{tooltip, Icon, IconName};
use crate::ui::molecules::{Placement, Popover};
use crate::ui::theme;

const EMOJI_FONT: &str = "Apple Color Emoji";

/// Picker key: the comment id, or "pr" for the PR itself.
fn key(subject: Option<&str>) -> String {
    subject.map(str::to_string).unwrap_or_else(|| "pr".into())
}

fn who(g: &ReactionGroup) -> String {
    let more = g.count as usize - g.users.len().min(g.count as usize);
    match (g.users.is_empty(), more) {
        (true, _) => format!("{} reactions", g.count),
        (false, 0) => g.users.join(", "),
        (false, n) => format!("{} and {n} more", g.users.join(", ")),
    }
}

impl GitViewer {
    /// Chips and the add button; `hover_group` shows the add button only while its group is hovered
    /// (it stays visible when there are chips). `can_add` is false where current reactions are unknown.
    pub(super) fn reactions_bar(
        &self,
        subject: Option<&str>,
        groups: &[ReactionGroup],
        hover_group: SharedString,
        can_add: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if groups.is_empty() && !can_add {
            return None;
        }
        let k = key(subject);
        let chips: Vec<AnyElement> = groups
            .iter()
            .map(|g| {
                let (sub, kind) = (subject.map(str::to_string), g.kind);
                div()
                    .id(SharedString::from(format!("rx-{k}-{:?}", g.kind)))
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_1)
                    .h(theme::SPACE_6)
                    .px(theme::SPACE_2)
                    .rounded(theme::RADIUS_FULL)
                    .border_1()
                    .border_color(if g.me {
                        theme::accent()
                    } else {
                        theme::border_subtle()
                    })
                    .when(g.me, |e| e.bg(theme::tint_bg(theme::accent())))
                    .when(can_add, |e| {
                        e.cursor_pointer().hover(|s| s.bg(theme::surface_hover()))
                    })
                    .tooltip(tooltip(who(g)))
                    .when(can_add, |e| {
                        e.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.toggle_reaction(sub.clone(), kind, cx)
                        }))
                    })
                    .child(
                        div()
                            .font_family(EMOJI_FONT)
                            .text_size(theme::text_xs())
                            .child(g.kind.emoji()),
                    )
                    .child(
                        div()
                            .text_size(theme::text_xs())
                            .text_color(if g.me {
                                theme::accent()
                            } else {
                                theme::text_muted()
                            })
                            .child(g.count.to_string()),
                    )
                    .into_any_element()
            })
            .collect();
        // Comments add reactions from their hover toolbar; the PR keeps the button beside its chips.
        let add = (can_add && subject.is_none()).then(|| {
            self.add_reaction_button_in(&k, None, groups.is_empty(), hover_group.clone(), cx)
        });
        Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(theme::SPACE_1)
                .children(chips)
                .children(add)
                .into_any_element(),
        )
    }

    /// "Add reaction" button of a comment's hover toolbar, with its picker.
    pub(super) fn add_reaction_button(
        &self,
        comment_id: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.add_reaction_button_in(
            comment_id,
            Some(comment_id),
            false,
            SharedString::default(),
            cx,
        )
    }

    /// The button opening the picker for `k`; `hide_until_hover` keeps it invisible until `hover_group` is hovered.
    fn add_reaction_button_in(
        &self,
        k: &str,
        subject: Option<&str>,
        hide_until_hover: bool,
        hover_group: SharedString,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let open = matches!(&self.layer, Some(Layer::Reactions(x)) if x == k);
        let k2 = k.to_string();
        let btn = div()
            .id(SharedString::from(format!("rx-add-{k}")))
            .flex()
            .items_center()
            .justify_center()
            .size(theme::control_h())
            .rounded(theme::RADIUS_SM)
            .cursor_pointer()
            .hover(|s| s.bg(theme::surface_hover()))
            .when(hide_until_hover && !open, |e| {
                e.opacity(0.).group_hover(hover_group, |s| s.opacity(1.))
            })
            .tooltip(tooltip("Add reaction"))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.toggle_layer(Layer::Reactions(k2.clone()), cx)
            }))
            .child(
                Icon::new(IconName::MoodPlus)
                    .size(theme::ICON_MD)
                    .color(theme::text_subtle()),
            );
        div()
            .relative()
            .child(btn)
            .children(open.then(|| self.reaction_picker(subject, cx)))
            .into_any_element()
    }

    fn reaction_picker(&self, subject: Option<&str>, cx: &mut Context<Self>) -> impl IntoElement {
        let k = key(subject);
        let provider = self
            .detail
            .as_ref()
            .map(|d| d.summary.id.repo.kind)
            .unwrap_or(gv_engine::ProviderKind::GitHub);
        let items = ReactionKind::for_provider(provider)
            .iter()
            .copied()
            .map(|kind| {
                let sub = subject.map(str::to_string);
                div()
                    .id(SharedString::from(format!("rx-pick-{k}-{kind:?}")))
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(theme::SPACE_8)
                    .rounded(theme::RADIUS_MD)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme::surface_hover()))
                    .font_family(EMOJI_FONT)
                    .text_size(theme::text_base())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.layer = None;
                        this.toggle_reaction(sub.clone(), kind, cx)
                    }))
                    .child(kind.emoji())
            });
        let panel = div()
            .flex()
            .p(theme::SPACE_1)
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_lg()
            .children(items);
        Popover::new(SharedString::from(format!("rx-picker-{k}")))
            .placement(Placement::BottomStart)
            .on_dismiss(cx.listener(|this, _: &MouseDownEvent, _, cx| this.dismiss_layer(cx)))
            .child(panel)
    }
}
