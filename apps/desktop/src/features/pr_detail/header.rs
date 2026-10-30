//! Review mode top bar, one line: back, state icon, the short read-only title (repo, number and branch on
//! hover), the unresolved count, then Review, Merge and the More menu. Repo, branch, CI and approvals live in
//! the Overview properties; +/- counts in the file tree header.

use gpui::{div, prelude::*, ClickEvent, Context, MouseDownEvent};
use gv_engine::ReviewState;

use crate::app::{GitViewer, Layer};
use crate::features::keymap::{
    self, CopyLink, HistoryBack, NextPr, OpenInBrowser, PrevPr, ToggleDraft,
};
use crate::features::shared::{layout, number_label, state_icon};
use crate::state::{Copied, DetailState};
use crate::ui::atoms::{tooltip, Button, ButtonVariant, HintAnchor, Icon, IconName, ShortcutHint};
use crate::ui::molecules::{DropdownMenu, MenuItem, MenuSeparator};
use crate::ui::theme;

/// `+a -d · N files` in muted mono.
pub(super) fn stats(s: &gv_engine::ChangeRequest) -> impl IntoElement {
    let files = s.changed_files.map(|n| {
        if n == 1 {
            "1 file".to_string()
        } else {
            format!("{n} files")
        }
    });
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(theme::space_1p5())
        .text_size(theme::text_xs())
        .font_family(theme::mono_font())
        .when_some(s.additions, |e, a| {
            e.child(div().text_color(theme::success()).child(format!("+{a}")))
        })
        .when_some(s.deletions, |e, d| {
            e.child(div().text_color(theme::danger()).child(format!("-{d}")))
        })
        .when_some(files, |e, f| {
            e.child(div().text_color(theme::text_subtle()).child("\u{00b7}"))
                .child(div().text_color(theme::text_muted()).child(f))
        })
}

/// Approvals shown once: given of needed, and whether anyone requested changes.
#[derive(Debug, PartialEq)]
pub(super) struct ApprovalCounts {
    pub given: u32,
    pub of: u32,
    pub changes: bool,
}

/// Reconciles the provider's approval rule (`approved_by` count, `required`) with reviewer states: an approver
/// need not be a requested reviewer, and reviewer states can lag, so given is the larger of both; the target is
/// the required count, else the reviewers (never below given). None when there is nothing to show.
pub(super) fn approval_counts(
    api: Option<gv_engine::Approvals>,
    reviewers: &[gv_engine::Review],
) -> Option<ApprovalCounts> {
    let by_state = reviewers
        .iter()
        .filter(|r| r.state == ReviewState::Approved)
        .count() as u32;
    let given = api.map(|a| a.given).unwrap_or(0).max(by_state);
    let of = match api {
        Some(a) if a.required > 0 => a.required,
        _ => (reviewers.len() as u32).max(given),
    };
    let changes = reviewers
        .iter()
        .any(|r| r.state == ReviewState::ChangesRequested);
    (of > 0).then_some(ApprovalCounts { given, of, changes })
}

impl GitViewer {
    /// One approvals indicator: "1/1" with a check (tooltip lists approvers), red when changes are requested.
    pub(super) fn approvals(&self, d: &DetailState) -> Option<impl IntoElement> {
        let s = &d.summary;
        let a = approval_counts(
            d.detail.as_ref().and_then(|x| x.review.approvals),
            &s.reviewers,
        )?;
        let color = if a.changes {
            theme::danger()
        } else if a.given >= a.of {
            theme::success()
        } else {
            theme::text_muted()
        };
        let names: Vec<&str> = s
            .reviewers
            .iter()
            .filter(|r| r.state == ReviewState::Approved)
            .map(|r| r.user.username.as_str())
            .collect();
        let mut tip = format!("{} of {} approvals", a.given, a.of);
        if !names.is_empty() {
            tip.push_str(&format!(": {}", names.join(", ")));
        }
        if a.changes {
            tip = format!("Changes requested. {tip}");
        }
        Some(
            div()
                .id("approvals")
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(theme::space_1())
                .text_size(theme::text_xs())
                .text_color(color)
                .tooltip(tooltip(tip))
                .child(
                    Icon::new(if a.changes {
                        IconName::AlertCircle
                    } else {
                        IconName::Check
                    })
                    .size(theme::icon_sm())
                    .color(color),
                )
                .child(format!("{}/{} approved", a.given, a.of)),
        )
    }

    pub(super) fn render_review_bar(
        &self,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let s = &d.summary;
        let back = Button::new("review-back", "")
            .icon(IconName::ChevronLeft)
            .variant(ButtonVariant::Ghost)
            .tooltip(keymap::tip::<HistoryBack>("Back to list"))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.leave_review(false, cx)));
        // One line: the short read-only title (edited at the top of the Overview), repo and branch on hover.
        let tip = format!(
            "{}\n{} {}  {} -> {}",
            s.title,
            self.repo_label(&s.id.repo),
            number_label(s),
            s.source_branch,
            s.target_branch
        );
        let title = div()
            .id("review-title")
            .px(theme::space_1())
            .min_w(theme::space_0())
            .truncate()
            .text_size(theme::text_base())
            .font_weight(theme::WEIGHT_SEMIBOLD)
            .tooltip(tooltip(tip))
            .child(s.title.clone());
        let heading = div()
            .flex()
            .items_center()
            .gap(theme::space_2())
            .flex_1()
            .min_w(theme::space_0())
            .child(title)
            .children(self.unresolved_count(d, cx));
        div()
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(theme::space_2p5())
            .h(layout::REVIEW_BAR_H)
            .pl(layout::TRAFFIC_LIGHTS_W)
            .pr(theme::space_3())
            .border_b_1()
            .border_color(theme::border_subtle())
            .child(ShortcutHint::wrap(
                self.hinted::<HistoryBack>(back, HintAnchor::Corner),
                self.hint_group(
                    "hint-review-pr-nav",
                    vec![self.hint_keys::<PrevPr>(), self.hint_keys::<NextPr>()],
                    HintAnchor::Below,
                ),
            ))
            .child(state_icon(s.state))
            .child(heading)
            .child(div().w(theme::space_1()))
            .children(self.render_review_button(d, cx))
            .children(self.render_merge_button(d, cx))
            .child(
                div()
                    .relative()
                    .child(ShortcutHint::wrap(
                        Button::new("detail-more", "")
                            .icon(IconName::Ellipsis)
                            .variant(ButtonVariant::Ghost)
                            .tooltip("More")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.toggle_layer(Layer::DetailMore, cx)
                            })),
                        // Menu rows show their own shortcuts once open.
                        self.hint_group(
                            "hint-more",
                            vec![
                                self.hint_keys::<OpenInBrowser>(),
                                self.hint_keys::<CopyLink>(),
                                self.hint_keys::<ToggleDraft>(),
                            ],
                            HintAnchor::Below,
                        )
                        .filter(|_| self.layer != Some(Layer::DetailMore)),
                    ))
                    .when(self.layer == Some(Layer::DetailMore), |el| {
                        el.child(self.render_detail_menu(d, cx))
                    }),
            )
    }

    /// "3 unresolved" (amber when some need me); a click jumps to the next one.
    fn unresolved_count(
        &self,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        let attn = crate::state::attention(d);
        let n = attn.iter().filter(|a| a.unresolved()).count();
        if n == 0 {
            return None;
        }
        let needs = attn
            .iter()
            .filter(|a| **a == gv_engine::Attention::NeedsMe)
            .count();
        let color = if needs > 0 {
            theme::warning()
        } else {
            theme::text_muted()
        };
        let tip = if needs > 0 {
            format!("{needs} need you. Next unresolved  u")
        } else {
            "Next unresolved  u".to_string()
        };
        Some(
            div()
                .id("unresolved-count")
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(theme::space_1())
                .px(theme::space_1())
                .rounded(theme::RADIUS_SM)
                .cursor_pointer()
                .hover(|s| s.bg(theme::surface_hover()))
                .text_size(theme::text_xs())
                .text_color(color)
                .tooltip(tooltip(tip))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.jump_unresolved(1, cx)))
                .child(
                    Icon::new(IconName::MessageCircle)
                        .size(theme::icon_sm())
                        .color(color),
                )
                .child(format!("{n} unresolved")),
        )
    }

    fn render_detail_menu(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let link = d.summary.web_url.clone();
        DropdownMenu::new("detail-more-menu")
            .on_dismiss(cx.listener(|this, _: &MouseDownEvent, _, cx| this.dismiss_layer(cx)))
            .child(
                MenuItem::new("menu-open", "Open in browser")
                    .icon(IconName::ExternalLink)
                    .shortcut(keymap::kbd::<OpenInBrowser>())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.layer = None;
                        if let Some(d) = &this.detail {
                            cx.open_url(&d.summary.web_url)
                        }
                    })),
            )
            .child(
                MenuItem::new("menu-copy-link", "Copy link")
                    .icon(IconName::Link)
                    .shortcut(keymap::kbd::<keymap::CopyLink>())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.layer = None;
                        this.copy(Copied::Link, link.clone(), cx)
                    })),
            )
            .child(
                MenuItem::new("menu-refresh", "Refresh")
                    .icon(IconName::Refresh)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.layer = None;
                        this.load_detail(cx)
                    })),
            )
            .children(self.edit_menu_items(d, cx))
            .child(MenuSeparator)
            .children(self.snooze_items(&d.summary, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_engine::{Approvals, Review, User};

    fn rev(state: ReviewState) -> Review {
        Review {
            user: User {
                username: "ada".into(),
                ..Default::default()
            },
            state,
        }
    }

    #[test]
    fn approvals_reconcile_rule_and_reviewer_states() {
        // Approver not among reviewers: the rule says 1 given, the reviewer is pending. One indicator, 1/1.
        let a = approval_counts(
            Some(Approvals {
                given: 1,
                required: 0,
            }),
            &[rev(ReviewState::Pending)],
        )
        .unwrap();
        assert_eq!((a.given, a.of), (1, 1));
        // Required approvals win as the target.
        let a = approval_counts(
            Some(Approvals {
                given: 0,
                required: 2,
            }),
            &[rev(ReviewState::Approved)],
        )
        .unwrap();
        assert_eq!((a.given, a.of), (1, 2));
        let a = approval_counts(
            None,
            &[
                rev(ReviewState::ChangesRequested),
                rev(ReviewState::Pending),
            ],
        )
        .unwrap();
        assert!(a.changes && a.given == 0 && a.of == 2);
        assert_eq!(
            approval_counts(
                Some(Approvals {
                    given: 0,
                    required: 0
                }),
                &[]
            ),
            None
        );
    }
}
