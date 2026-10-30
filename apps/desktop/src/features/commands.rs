//! Handlers for the window commands added with the keymap: palette, toggles, history, review.

use gpui::{prelude::*, Context, Window};
use gv_engine::{ChangeRequestId, ChangeRequestState, ReviewAction};

use super::keymap::*;
use crate::app::{GitViewer, Overlay};
use crate::state::cmd_ctx::{CmdCtx, Gate, PrAct};
use crate::state::pr_nav::{adjacent, adjacent_section, Step};
use crate::state::{Copied, DetailTab};

impl GitViewer {
    pub(crate) fn command_handlers<E: InteractiveElement>(
        &self,
        el: E,
        cx: &mut Context<Self>,
    ) -> E {
        el.on_action(
            cx.listener(|this, _: &CommandPalette, window, cx| this.open_palette(window, cx)),
        )
        .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
        .on_action(cx.listener(|this, _: &ToggleDetail, _, cx| this.toggle_detail(cx)))
        .on_action(cx.listener(|this, _: &ToggleTheme, _, cx| this.toggle_theme(cx)))
        .on_action(cx.listener(Self::close_detail_or_window))
        .on_action(cx.listener(|this, _: &GoInbox, _, cx| this.go_view(1, cx)))
        .on_action(cx.listener(|this, _: &GoStarred, _, cx| this.go_view(2, cx)))
        .on_action(cx.listener(|this, _: &GoProject1, _, cx| this.go_view(3, cx)))
        .on_action(cx.listener(|this, _: &GoProject2, _, cx| this.go_view(4, cx)))
        .on_action(cx.listener(|this, _: &GoProject3, _, cx| this.go_view(5, cx)))
        .on_action(cx.listener(|this, _: &GoProject4, _, cx| this.go_view(6, cx)))
        .on_action(cx.listener(|this, _: &GoProject5, _, cx| this.go_view(7, cx)))
        .on_action(cx.listener(|this, _: &GoProject6, _, cx| this.go_view(8, cx)))
        .on_action(cx.listener(|this, _: &GoProject7, _, cx| this.go_view(9, cx)))
        .on_action(cx.listener(|this, _: &NextPr, _, cx| this.step_pr(1, false, cx)))
        .on_action(cx.listener(|this, _: &PrevPr, _, cx| this.step_pr(-1, false, cx)))
        .on_action(cx.listener(|this, _: &NextSection, _, cx| this.step_pr(1, true, cx)))
        .on_action(cx.listener(|this, _: &PrevSection, _, cx| this.step_pr(-1, true, cx)))
        .on_action(
            cx.listener(|this, _: &ShowShortcuts, window, cx| this.open_shortcuts(window, cx)),
        )
        .on_action(cx.listener(|this, _: &CopyBranch, _, cx| {
            if let Some(c) = this.target_item(cx) {
                this.copy(Copied::Branch, c.source_branch, cx)
            }
        }))
        .on_action(cx.listener(|this, _: &HistoryBack, _, cx| this.history_step(-1, cx)))
        .on_action(cx.listener(|this, _: &HistoryForward, _, cx| this.history_step(1, cx)))
        .on_action(cx.listener(|this, _: &CopyLink, _, cx| {
            if let Some(c) = this.target_item(cx) {
                this.copy(Copied::Link, c.web_url, cx)
            }
        }))
        .on_action(cx.listener(|this, _: &PostComment, _, cx| {
            if let Overlay::ReviewSummary(a) = this.overlay {
                this.submit_review(a, cx)
            } else if this.detail.is_some() {
                this.post_comment(cx)
            }
        }))
        .on_action(cx.listener(|this, _: &Approve, _, cx| {
            if this.gate(PrAct::Approve, cx) {
                this.submit_review(ReviewAction::Approve, cx)
            }
        }))
        .on_action(cx.listener(|this, _: &RequestChanges, window, cx| {
            if this.gate(PrAct::RequestChanges, cx) {
                this.open_review_summary(ReviewAction::RequestChanges, window, cx)
            }
        }))
        .on_action(cx.listener(|this, _: &MergePr, _, cx| {
            if this.gate(PrAct::Merge, cx) {
                this.open_merge(cx)
            }
        }))
        .on_action(cx.listener(|this, _: &ToggleDraft, _, cx| {
            if this.gate(PrAct::ToggleDraft, cx) {
                let draft = this
                    .detail
                    .as_ref()
                    .is_some_and(|d| d.summary.state == ChangeRequestState::Draft);
                this.set_draft(!draft, cx)
            }
        }))
        .on_action(cx.listener(|this, _: &ClosePr, _, cx| {
            if this.gate(PrAct::Close, cx) {
                let reopen = this
                    .detail
                    .as_ref()
                    .and_then(|d| d.detail.as_ref())
                    .is_some_and(|x| x.edit.caps.reopen);
                if reopen {
                    this.set_open(true, cx)
                } else {
                    this.layer = None;
                    this.overlay = Overlay::ConfirmClose;
                    cx.notify()
                }
            }
        }))
        .on_action(cx.listener(|this, _: &ToggleFileTree, _, cx| {
            if this.reviewing {
                this.toggle_tree(cx)
            }
        }))
        .on_action(cx.listener(|this, _: &ToggleRightPanel, _, cx| {
            if this.reviewing {
                this.toggle_right(cx)
            }
        }))
        .on_action(cx.listener(|this, _: &QuickOpenFile, _, cx| this.open_quick_file(cx)))
        .on_action(cx.listener(|this, _: &ToggleDiffView, _, cx| {
            if this.reviewing {
                let split = !this.settings.diff_split;
                this.set_split(split, cx)
            }
        }))
        .on_action(cx.listener(|this, _: &NextUnresolved, _, cx| {
            if this.reviewing {
                this.jump_unresolved(1, cx)
            }
        }))
        .on_action(cx.listener(|this, _: &PrevUnresolved, _, cx| {
            if this.reviewing {
                this.jump_unresolved(-1, cx)
            }
        }))
        .on_action(cx.listener(|this, _: &ToggleViewed, _, cx| {
            if this.reviewing {
                if let Some(cur) = this.detail.as_ref().map(|d| d.code.current_file()) {
                    this.toggle_viewed(cur, cx)
                }
            }
        }))
    }

    /// Snapshot for command availability (palette rows, gated shortcuts).
    pub(crate) fn cmd_ctx(&self) -> CmdCtx {
        let d = self.detail.as_ref();
        CmdCtx {
            pr_open: d.is_some(),
            reviewing: self.reviewing && d.is_some(),
            diff: self.reviewing
                && d.is_some_and(|d| {
                    d.tab == DetailTab::Code && d.code.files.as_ref().is_some_and(|f| !f.is_empty())
                }),
            projects: self.projects.len(),
            gate: d.and_then(|d| {
                let x = d.detail.as_ref()?;
                Some(Gate {
                    review: x.review.caps.clone(),
                    edit: x.edit.caps.clone(),
                })
            }),
        }
    }

    /// True when `a` is possible on the open PR; otherwise a toast says why.
    fn gate(&mut self, a: PrAct, cx: &mut Context<Self>) -> bool {
        match self.cmd_ctx().deny(a) {
            None => true,
            Some(why) => {
                self.show_info_toast(why, cx);
                false
            }
        }
    }

    /// Cmd+Up/Down (or with Option, by section): the adjacent PR of the visible list, no wrap.
    /// Triage moves the selection and preview; Review mode opens that PR in Review mode.
    pub(crate) fn step_pr(&mut self, delta: isize, by_section: bool, cx: &mut Context<Self>) {
        let sections = self.nav_sections(cx);
        let current = if self.reviewing {
            self.detail.as_ref().map(|d| d.summary.id.clone())
        } else {
            self.cursor
                .clone()
                .or_else(|| self.detail.as_ref().map(|d| d.summary.id.clone()))
        };
        let step = if by_section {
            adjacent_section(&sections, current.as_ref(), delta)
        } else {
            adjacent(&sections, current.as_ref(), delta)
        };
        match step {
            Step::To(id) => self.go_to_pr(id, cx),
            Step::End => {
                let what = if by_section { "section" } else { "PR" };
                let end = if delta > 0 { "Last" } else { "First" };
                self.show_info_toast(format!("{end} {what} in {}", self.project_name()), cx)
            }
        }
    }

    /// Select `id`; in Review mode open it there (fresh scroll, same diff view), else preview it.
    pub(crate) fn go_to_pr(&mut self, id: ChangeRequestId, cx: &mut Context<Self>) {
        let Some(c) = self.find_item(&id) else { return };
        self.set_cursor(id.clone(), cx);
        self.reveal_cursor.set(true);
        if self.reviewing {
            self.remember_position();
            self.review.forget(&id);
            self.open_detail(c, cx);
            self.enter_review(cx);
        } else if !self.preview_hidden {
            self.open_detail(c, cx);
        }
        cx.notify();
    }

    /// Cmd+W closes the topmost thing: an overlay, Review mode, the preview, else the window.
    fn close_detail_or_window(
        &mut self,
        _: &CloseDetail,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.overlay != Overlay::None {
            self.overlay = Overlay::None;
            window.focus(&self.focus);
            cx.notify();
        } else if self.reviewing {
            self.exit_review(cx);
        } else if self.detail.is_some() {
            self.close_detail(cx);
        } else {
            window.remove_window();
        }
    }
}
