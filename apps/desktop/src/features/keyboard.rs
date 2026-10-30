//! Window-level shortcuts: list navigation, dismissal and PR actions.
//!
//! Letter keys only fire outside text inputs. Keyboard moves are never animated.

use gpui::{prelude::*, App, Context, Focusable, Window};
use gv_engine::{ChangeRequest, Involvement};

use crate::app::{GitViewer, Overlay};
use crate::features::pr_detail::checkout_command;
use crate::state::Copied;

pub use super::keymap::*;

pub fn bind_keys(cx: &mut App) {
    super::keymap::bind_keys(cx);
    super::code_keys::bind_keys(cx);
    super::keymap::bind_find_keys(cx);
}

impl GitViewer {
    /// Attach the handlers to the root element.
    pub(crate) fn key_handlers<E: InteractiveElement>(&self, el: E, cx: &mut Context<Self>) -> E {
        let el = self.command_handlers(self.code_key_handlers(el, cx), cx);
        el.key_context("GitViewer")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &SelectNext, _, cx| this.move_cursor(1, cx)))
            .on_action(cx.listener(|this, _: &SelectPrev, _, cx| this.move_cursor(-1, cx)))
            .on_action(cx.listener(|this, _: &OpenSelected, window, cx| {
                if this.code_tab_active() && this.has_line_selection() {
                    this.comment_on_selection(window, cx)
                } else if this.reviewing {
                    // Already reviewing.
                } else if let Some(c) = this.cursor_item(cx) {
                    if this.detail.as_ref().map(|d| &d.summary.id) != Some(&c.id) {
                        this.open_detail(c, cx);
                    }
                    this.enter_review(cx)
                } else if this.detail.is_some() {
                    this.enter_review(cx)
                }
            }))
            .on_action(cx.listener(Self::dismiss))
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| {
                // Review mode: find in the diff; triage: the list search.
                if this.reviewing && this.detail.is_some() {
                    this.open_find(window, cx)
                } else {
                    window.focus(&this.search.focus_handle(cx))
                }
            }))
            .on_action(cx.listener(|this, _: &Refresh, _, cx| this.refresh(cx)))
            .on_action(cx.listener(|this, _: &OpenInBrowser, _, cx| {
                if let Some(c) = this.target_item(cx) {
                    cx.open_url(&c.web_url)
                }
            }))
            .on_action(cx.listener(|this, _: &CopyCheckout, window, cx| {
                if this.code_tab_active() && this.has_line_selection() {
                    this.comment_on_selection(window, cx)
                } else if let Some(c) = this.target_item(cx) {
                    this.copy(Copied::Checkout, checkout_command(&c.id), cx)
                }
            }))
            .on_action(cx.listener(|this, _: &ToggleStar, _, cx| {
                if let Some(c) = this.target_item(cx) {
                    this.toggle_star(c.id, cx)
                }
            }))
            .on_action(
                cx.listener(|this, _: &TabAuthored, _, cx| this.set_tab(Involvement::Authored, cx)),
            )
            .on_action(
                cx.listener(|this, _: &TabReviewing, _, cx| {
                    this.set_tab(Involvement::Reviewing, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &TabAll, _, cx| this.set_tab(Involvement::All, cx)))
            .on_action(cx.listener(|this, _: &NextFile, _, cx| {
                if this.code_tab_active() {
                    this.step_file(1, cx)
                }
            }))
            .on_action(cx.listener(|this, _: &PrevFile, _, cx| {
                if this.code_tab_active() {
                    this.step_file(-1, cx)
                }
            }))
            .on_action(cx.listener(|this, _: &ToggleFile, _, cx| {
                if this.code_tab_active() {
                    this.toggle_current_file(cx)
                }
            }))
            .on_action(cx.listener(|_, _: &Minimize, window, _| window.minimize_window()))
            .on_action(cx.listener(|_, _: &Zoom, window, _| window.zoom_window()))
            .on_action(cx.listener(|_, _: &About, window, cx| {
                let detail = format!(
                    "Version {}\nPRs and MRs across GitHub, GitLab and Azure DevOps.",
                    env!("CARGO_PKG_VERSION")
                );
                drop(window.prompt(
                    gpui::PromptLevel::Info,
                    "Confluo",
                    Some(&detail),
                    &["OK"],
                    cx,
                ));
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| this.toggle_settings(cx)))
    }

    /// Code tab shortcuts (n/p, ]/[, x) act only while the diff is showing.
    fn code_tab_active(&self) -> bool {
        self.reviewing
            && self
                .detail
                .as_ref()
                .is_some_and(|d| d.tab == crate::state::DetailTab::Code && d.code.viewer.is_none())
    }

    pub(crate) fn cursor_item(&self, cx: &App) -> Option<ChangeRequest> {
        let id = self.cursor.as_ref()?;
        self.items
            .iter()
            .find(|c| &c.id == id)
            .cloned()
            .filter(|_| self.nav_order(cx).contains(id))
    }

    /// The PR a shortcut acts on: the open detail, else the keyboard selection.
    pub(crate) fn target_item(&self, cx: &App) -> Option<ChangeRequest> {
        self.detail
            .as_ref()
            .map(|d| d.summary.clone())
            .or_else(|| self.cursor_item(cx))
    }

    pub(crate) fn move_cursor(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.reviewing {
            return;
        }
        let order = self.nav_order(cx);
        if order.is_empty() {
            return;
        }
        let at = self
            .previewed()
            .and_then(|id| order.iter().position(|x| x == id));
        let next = crate::state::step_selection(at, delta, order.len());
        self.set_cursor(order[next].clone(), cx);
        self.reveal_cursor.set(true);
        // The keyboard highlight is the preview: moving it opens that PR.
        self.preview_hidden = false;
        if let Some(c) = self.cursor_item(cx) {
            self.open_detail(c, cx);
        }
        cx.notify();
    }

    /// Escape closes the topmost thing: menu, dialog, detail, then the search text.
    fn dismiss(&mut self, _: &Dismiss, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_layers(cx) {
            // Closed the open menu, popover or picker.
        } else if self.discard_prompt {
            self.keep_editing(cx);
            return;
        } else if self.overlay != Overlay::None {
            // Same as a press on the scrim: dialogs with typed input ask first.
            self.request_close_overlay(window, cx);
            if self.discard_prompt {
                return;
            }
        } else if self.reviewing && self.find.open {
            self.find.open = false;
        } else if self.reviewing && self.clear_commit(cx) {
            // Back from one commit to all changes.
        } else if self.reviewing
            && self
                .detail
                .as_ref()
                .is_some_and(|d| d.code.viewer.as_ref().is_some_and(|v| v.goto))
        {
            if let Some(v) = self.detail.as_mut().and_then(|d| d.code.viewer.as_mut()) {
                v.goto = false;
            }
        } else if self.cancel_edits(cx) {
            // Left title or description editing, or closed the people picker.
        } else if self.code_tab_active() && self.cancel_inline(cx) {
            // Closed the inline comment composer; keep the detail open.
        } else if self.code_lines_active() && self.clear_line_selection(cx) {
            // Cleared the line selection.
        } else if self.reviewing && self.review.quick_open {
            self.review.quick_open = false;
        } else if self.reviewing && self.tree_filter.focus_handle(cx).is_focused(window) {
            self.tree_filter.update(cx, |i, cx| i.set_text("", cx));
        } else if self.reviewing {
            self.leave_review(false, cx);
        } else if self.search.focus_handle(cx).is_focused(window) || self.detail.is_none() {
            self.search.update(cx, |i, cx| i.set_text("", cx));
            self.cursor = None;
        } else {
            // Esc clears the preview and the selection together.
            self.close_detail(cx);
            self.cursor = None;
        }
        window.focus(&self.focus);
        cx.notify();
    }

    fn set_tab(&mut self, tab: Involvement, cx: &mut Context<Self>) {
        self.switch_list_tab(crate::state::list_tabs_index(tab), true, cx);
    }
}
