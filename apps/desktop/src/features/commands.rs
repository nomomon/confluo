//! Handlers for the window commands added with the keymap: palette, toggles, history, review.

use gpui::{prelude::*, Context, Window};
use gv_engine::ReviewAction;

use super::keymap::*;
use crate::app::{GitViewer, Overlay};
use crate::state::Copied;

impl GitViewer {
    pub(crate) fn command_handlers<E: InteractiveElement>(&self, el: E, cx: &mut Context<Self>) -> E {
        el.on_action(cx.listener(|this, _: &CommandPalette, window, cx| this.open_palette(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(cx.listener(|this, _: &ToggleDetail, _, cx| this.toggle_detail(cx)))
            .on_action(cx.listener(|this, _: &ToggleTheme, _, cx| this.toggle_theme(cx)))
            .on_action(cx.listener(Self::close_detail_or_window))
            .on_action(cx.listener(|this, _: &GoInbox, _, cx| this.go_view(1, cx)))
            .on_action(cx.listener(|this, _: &GoStarred, _, cx| this.go_view(2, cx)))
            .on_action(cx.listener(|this, _: &GoProject, _, cx| this.go_view(3, cx)))
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
            .on_action(cx.listener(|this, _: &Approve, _, cx| this.submit_review(ReviewAction::Approve, cx)))
    }

    /// Cmd+W closes the topmost thing: an overlay, the detail pane, else the window.
    fn close_detail_or_window(&mut self, _: &CloseDetail, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlay != Overlay::None {
            self.overlay = Overlay::None;
            window.focus(&self.focus);
            cx.notify();
        } else if self.detail.is_some() {
            self.close_detail(cx);
        } else {
            window.remove_window();
        }
    }
}
