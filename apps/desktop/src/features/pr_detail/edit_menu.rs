//! Edit entry points in the detail header: the inline title, the Merge button and the
//! edit items of the More menu (draft, copy, close/reopen), all driven by capabilities.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};
use gv_engine::ChangeRequestState;

use crate::app::{GitViewer, Overlay};
use crate::features::keymap::{self, MergePr, ToggleDraft};
use crate::state::{Copied, DetailState};
use crate::ui::atoms::{tooltip, Button, IconName};
use crate::ui::molecules::{MenuItem, MenuSeparator};
use crate::ui::theme;

impl GitViewer {
    /// Big title; click to edit when allowed (Enter saves, Esc cancels).
    pub(super) fn render_title(&self, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let base = || div().flex_1().min_w(theme::SPACE_0).truncate().text_size(theme::text_base()).font_weight(theme::WEIGHT_SEMIBOLD);
        if d.ui.editing_title {
            let field = div().rounded(theme::RADIUS_MD).border_1().border_color(theme::focus_ring()).bg(theme::surface()).py(theme::SPACE_1).child(self.title_input.clone());
            return base().child(field).into_any_element();
        }
        let editable = d.detail.as_ref().is_some_and(|x| x.edit.caps.title);
        let title = div().id("detail-title").px(theme::SPACE_1).rounded(theme::RADIUS_MD).truncate().child(d.summary.title.clone());
        if !editable {
            return base().child(title).into_any_element();
        }
        let title = title
            .cursor_text()
            .hover(|s| s.bg(theme::surface_hover()))
            .tooltip(tooltip("Click to edit the title"))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.start_title_edit(window, cx)));
        base().child(title).into_any_element()
    }

    /// "Merge" beside the Review button when the provider says it is possible now.
    pub(super) fn render_merge_button(&self, d: &DetailState, cx: &mut Context<Self>) -> Option<AnyElement> {
        let caps = &d.detail.as_ref()?.edit.caps;
        caps.merge.then(|| {
            Button::new("merge-open", "Merge").icon(IconName::GitMerge).tooltip(keymap::tip::<MergePr>("Merge")).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.open_merge(cx))).into_any_element()
        })
    }

    /// Edit items for the More menu.
    pub(super) fn edit_menu_items(&self, d: &DetailState, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(det) = d.detail.as_ref() else { return vec![] };
        let c = &det.edit.caps;
        let mut out: Vec<AnyElement> = vec![];
        let draft = d.summary.state == ChangeRequestState::Draft;
        if c.draft {
            let (label, icon) = if draft { ("Ready for review", IconName::Eye) } else { ("Mark as draft", IconName::GitPullRequestDraft) };
            out.push(MenuItem::new("menu-draft", label).icon(icon).shortcut(keymap::kbd::<ToggleDraft>()).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_draft(!draft, cx))).into_any_element());
        }
        if c.title {
            out.push(MenuItem::new("menu-edit-title", "Edit title").icon(IconName::Pencil).on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.menu = None;
                this.start_title_edit(window, cx)
            })).into_any_element());
        }
        let branch = d.summary.source_branch.clone();
        out.push(MenuItem::new("menu-copy-branch", "Copy branch name").icon(IconName::Copy).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.menu = None;
            this.copy(Copied::Branch, branch.clone(), cx)
        })).into_any_element());
        if let Some(why) = c.merge_blocked.clone().filter(|_| !c.merge) {
            out.push(MenuItem::new("menu-merge", "Merge").icon(IconName::GitMerge).shortcut(keymap::kbd::<MergePr>()).disabled(true).tooltip(why).into_any_element());
        }
        if c.close || c.reopen {
            out.push(MenuSeparator.into_any_element());
        }
        if c.reopen {
            out.push(MenuItem::new("menu-reopen", "Reopen").icon(IconName::GitPullRequest).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.set_open(true, cx))).into_any_element());
        }
        if c.close {
            out.push(MenuItem::new("menu-close", "Close").icon(IconName::GitPullRequestClosed).destructive().on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.menu = None;
                this.overlay = Overlay::ConfirmClose;
                cx.notify()
            })).into_any_element());
        }
        out
    }
}
