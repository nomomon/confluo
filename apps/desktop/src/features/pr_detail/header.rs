//! Review mode top bar: back, state icon, the title with a muted line under it (repo and number,
//! branches, stats, CI, approvals), then Review, Merge and the More menu.

use gpui::{div, prelude::*, ClickEvent, Context, MouseButton, MouseDownEvent, SharedString};
use gv_engine::ReviewState;

use crate::app::{GitViewer, Menu};
use crate::features::keymap::{self, CopyLink, HistoryBack, NextPr, OpenInBrowser, PrevPr, ToggleDraft};
use crate::features::shared::{layout, number_label, state_icon};
use crate::state::{Copied, DetailState};
use crate::ui::atoms::{tooltip, Button, ButtonVariant, HintAnchor, Icon, IconName, ShortcutHint};
use crate::ui::molecules::{DropdownMenu, MenuItem, MenuSeparator};
use crate::ui::theme;

/// `+a -d · N files` in muted mono.
pub(super) fn stats(s: &gv_engine::ChangeRequest) -> impl IntoElement {
    let files = s.changed_files.map(|n| if n == 1 { "1 file".to_string() } else { format!("{n} files") });
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(theme::SPACE_1P5)
        .text_size(theme::text_xs())
        .font_family(theme::mono_font())
        .when_some(s.additions, |e, a| e.child(div().text_color(theme::success()).child(format!("+{a}"))))
        .when_some(s.deletions, |e, d| e.child(div().text_color(theme::danger()).child(format!("-{d}"))))
        .when_some(files, |e, f| e.child(div().text_color(theme::text_subtle()).child("\u{00b7}")).child(div().text_color(theme::text_muted()).child(f)))
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
pub(super) fn approval_counts(api: Option<gv_engine::Approvals>, reviewers: &[gv_engine::Review]) -> Option<ApprovalCounts> {
    let by_state = reviewers.iter().filter(|r| r.state == ReviewState::Approved).count() as u32;
    let given = api.map(|a| a.given).unwrap_or(0).max(by_state);
    let of = match api {
        Some(a) if a.required > 0 => a.required,
        _ => (reviewers.len() as u32).max(given),
    };
    let changes = reviewers.iter().any(|r| r.state == ReviewState::ChangesRequested);
    (of > 0).then_some(ApprovalCounts { given, of, changes })
}

impl GitViewer {
    /// One approvals indicator: "1/1" with a check (tooltip lists approvers), red when changes are requested.
    pub(super) fn approvals(&self, d: &DetailState) -> Option<impl IntoElement> {
        let s = &d.summary;
        let a = approval_counts(d.detail.as_ref().and_then(|x| x.review.approvals), &s.reviewers)?;
        let color = if a.changes { theme::danger() } else if a.given >= a.of { theme::success() } else { theme::text_muted() };
        let names: Vec<&str> = s.reviewers.iter().filter(|r| r.state == ReviewState::Approved).map(|r| r.user.username.as_str()).collect();
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
                .gap(theme::SPACE_1)
                .text_size(theme::text_xs())
                .text_color(color)
                .tooltip(tooltip(tip))
                .child(Icon::new(if a.changes { IconName::AlertCircle } else { IconName::Check }).size(theme::ICON_SM).color(color))
                .child(format!("{}/{} approved", a.given, a.of)),
        )
    }

    pub(super) fn render_review_bar(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let s = &d.summary;
        let repo = s.id.repo.clone();
        let branch = s.source_branch.clone();
        let copied = self.copied == Some(Copied::Branch);
        let back = Button::new("review-back", "").icon(IconName::ChevronLeft).variant(ButtonVariant::Ghost).tooltip(keymap::tip::<HistoryBack>("Back to list")).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.exit_review(cx)));
        let repo_el = div()
            .id("review-repo")
            .flex_shrink_0()
            .whitespace_nowrap()
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
            .tooltip(tooltip(format!("{}  right-click to rename", s.id.repo.path)))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, ev: &MouseDownEvent, _, cx| this.open_repo_menu(repo.clone(), ev.position, cx)))
            .child(format!("{} {}", self.repo_label(&s.id.repo), number_label(s)));
        let branch_el = div()
            .id("review-branch")
            .flex()
            .min_w(theme::SPACE_0)
            .flex_shrink()
            .items_center()
            .gap(theme::SPACE_1)
            .px(theme::SPACE_1)
            .rounded(theme::RADIUS_SM)
            .cursor_pointer()
            .hover(|st| st.bg(theme::surface_hover()))
            .font_family(theme::mono_font())
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
            .tooltip(tooltip(if copied { "Copied" } else { "Copy source branch" }))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.copy(Copied::Branch, branch.clone(), cx)))
            .child(div().min_w(theme::SPACE_0).truncate().child(SharedString::from(format!("{} -> {}", s.source_branch, s.target_branch))));
        // Title on the first line (ellipsis), everything else on one muted line under it, so nothing overflows.
        let meta = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2P5)
            .min_w(theme::SPACE_0)
            .overflow_hidden()
            .child(crate::features::shared::provider_mark(s.id.repo.kind))
            .child(repo_el)
            .child(branch_el)
            .child(stats(s))
            .children(self.render_ci(d, "review", cx))
            .children(self.approvals(d));
        let heading = div().flex().flex_col().flex_1().min_w(theme::SPACE_0).gap(theme::SPACE_0P5).child(div().flex().min_w(theme::SPACE_0).child(self.render_title(d, cx))).child(div().pl(theme::SPACE_1).child(meta));
        div()
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(theme::SPACE_2P5)
            .h(layout::REVIEW_BAR_H)
            .pl(layout::TRAFFIC_LIGHTS_W)
            .pr(theme::SPACE_3)
            .border_b_1()
            .border_color(theme::border_subtle())
            .child(ShortcutHint::wrap(
                self.hinted::<HistoryBack>(back, HintAnchor::Corner),
                self.hint_group("hint-review-pr-nav", vec![self.hint_keys::<PrevPr>(), self.hint_keys::<NextPr>()], HintAnchor::Below),
            ))
            .child(state_icon(s.state))
            .child(heading)
            .child(div().w(theme::SPACE_1))
            .children(self.render_review_button(d, cx))
            .children(self.render_merge_button(d, cx))
            .child(ShortcutHint::wrap(
                Button::new("detail-more", "").icon(IconName::Ellipsis).variant(ButtonVariant::Ghost).tooltip("More").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.menu = if this.menu == Some(Menu::DetailMore) { None } else { Some(Menu::DetailMore) };
                    cx.notify()
                })),
                // Menu rows show their own shortcuts once open.
                self.hint_group("hint-more", vec![self.hint_keys::<OpenInBrowser>(), self.hint_keys::<CopyLink>(), self.hint_keys::<ToggleDraft>()], HintAnchor::Below).filter(|_| self.menu != Some(Menu::DetailMore)),
            ))
            .when(self.menu == Some(Menu::DetailMore), |el| el.child(self.render_detail_menu(d, cx)))
    }

    fn render_detail_menu(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let link = d.summary.web_url.clone();
        DropdownMenu::new("detail-more-menu")
            .top(layout::REVIEW_BAR_H - theme::SPACE_1)
            .right(theme::SPACE_3)
            .on_dismiss(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                this.menu = None;
                cx.notify()
            }))
            .child(MenuItem::new("menu-open", "Open in browser").icon(IconName::ExternalLink).shortcut(keymap::kbd::<OpenInBrowser>()).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.menu = None;
                if let Some(d) = &this.detail {
                    cx.open_url(&d.summary.web_url)
                }
            })))
            .child(MenuItem::new("menu-copy-link", "Copy link").icon(IconName::Link).shortcut(keymap::kbd::<keymap::CopyLink>()).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.menu = None;
                this.copy(Copied::Link, link.clone(), cx)
            })))
            .child(MenuItem::new("menu-refresh", "Refresh").icon(IconName::Refresh).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.menu = None;
                this.load_detail(cx)
            })))
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
        Review { user: User { username: "ada".into(), ..Default::default() }, state }
    }

    #[test]
    fn approvals_reconcile_rule_and_reviewer_states() {
        // Approver not among reviewers: the rule says 1 given, the reviewer is pending. One indicator, 1/1.
        let a = approval_counts(Some(Approvals { given: 1, required: 0 }), &[rev(ReviewState::Pending)]).unwrap();
        assert_eq!((a.given, a.of), (1, 1));
        // Required approvals win as the target.
        let a = approval_counts(Some(Approvals { given: 0, required: 2 }), &[rev(ReviewState::Approved)]).unwrap();
        assert_eq!((a.given, a.of), (1, 2));
        let a = approval_counts(None, &[rev(ReviewState::ChangesRequested), rev(ReviewState::Pending)]).unwrap();
        assert!(a.changes && a.given == 0 && a.of == 2);
        assert_eq!(approval_counts(Some(Approvals { given: 0, required: 0 }), &[]), None);
    }
}
