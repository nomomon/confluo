//! Review mode top bar, one calm row: back, state icon, title, repo and number, branches,
//! stats, CI, approvals, then Review, Merge and the More menu.

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

impl GitViewer {
    /// "1/2" with a check: approvals given of required, else approved of requested reviewers.
    pub(super) fn approvals(&self, d: &DetailState) -> Option<impl IntoElement> {
        let s = &d.summary;
        let (given, of) = match d.detail.as_ref().and_then(|x| x.review.approvals) {
            Some(a) if a.required > 0 => (a.given, a.required),
            _ => (s.reviewers.iter().filter(|r| r.state == ReviewState::Approved).count() as u32, s.reviewers.len() as u32),
        };
        let changes = s.reviewers.iter().any(|r| r.state == ReviewState::ChangesRequested);
        (of > 0).then(|| {
            let color = if changes { theme::danger() } else if given >= of { theme::success() } else { theme::text_muted() };
            let tip = if changes { "Changes requested".to_string() } else { format!("{given} of {of} approved") };
            div()
                .id("approvals")
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(theme::SPACE_1)
                .text_size(theme::text_xs())
                .text_color(color)
                .tooltip(tooltip(tip))
                .child(Icon::new(if changes { IconName::AlertCircle } else { IconName::Check }).size(theme::ICON_SM).color(color))
                .child(format!("{given}/{of}"))
        })
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
        div()
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(theme::SPACE_2P5)
            .h(layout::TITLEBAR_INSET)
            .pl(layout::TRAFFIC_LIGHTS_W)
            .pr(theme::SPACE_3)
            .border_b_1()
            .border_color(theme::border_subtle())
            .child(ShortcutHint::wrap(
                self.hinted::<HistoryBack>(back, HintAnchor::Corner),
                self.hint_group("hint-review-pr-nav", vec![self.hint_keys::<PrevPr>(), self.hint_keys::<NextPr>()], HintAnchor::Below),
            ))
            .child(state_icon(s.state))
            .child(div().flex().flex_1().min_w(theme::SPACE_8 * 4.).child(self.render_title(d, cx)))
            .child(repo_el)
            .child(branch_el)
            .child(stats(s))
            .children(self.render_ci(d, "review", cx))
            .children(self.approvals(d))
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
            .top(layout::TITLEBAR_INSET - theme::SPACE_1)
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
