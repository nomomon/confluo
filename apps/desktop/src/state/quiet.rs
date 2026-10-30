//! Stale PRs and local snoozes: what the list leaves out and the undo for a snooze.

use gpui::Context;
use gv_engine::{ChangeRequest, ChangeRequestId, Snooze, SnoozeKind};

use super::err_str;
use crate::app::GitViewer;

/// List state for PRs kept out of the sections.
#[derive(Default)]
pub struct QuietState {
    /// Inactive longer than the stale setting, newest first.
    pub stale: Vec<ChangeRequest>,
    /// Show the collapsed "Older" section at the bottom.
    pub show_stale: bool,
    /// How many PRs a snooze or hide leaves out.
    pub snoozed: usize,
    /// The last snooze, for the toast's Undo.
    pub undo: Option<(ChangeRequestId, Option<Snooze>)>,
}

impl GitViewer {
    /// Snooze or hide a PR locally; the toast offers Undo.
    pub(crate) fn snooze(&mut self, cr: &ChangeRequest, kind: SnoozeKind, cx: &mut Context<Self>) {
        self.layer = None;
        match self.engine.set_snooze(cr, Some(kind)) {
            Ok(prev) => {
                let text = match kind {
                    SnoozeKind::Week => "Snoozed for a week",
                    SnoozeKind::UntilUpdated => "Snoozed until updated",
                    SnoozeKind::Hidden => "Pull request hidden",
                };
                if self.detail.as_ref().is_some_and(|d| d.summary.id == cr.id) {
                    self.close_detail(cx);
                }
                self.show_toast(text, cx);
                self.quiet.undo = Some((cr.id.clone(), prev));
            }
            Err(e) => self.error = Some(err_str(e)),
        }
        self.reload();
        cx.notify();
    }

    /// Undo the last snooze from the toast.
    pub(crate) fn undo_snooze(&mut self, cx: &mut Context<Self>) {
        if let Some((id, prev)) = self.quiet.undo.take() {
            if let Err(e) = self.engine.restore_snooze(&id, prev) {
                self.error = Some(err_str(e));
            }
            self.toast = None;
            self.reload();
            cx.notify();
        }
    }

    pub(crate) fn toggle_show_stale(&mut self, cx: &mut Context<Self>) {
        self.quiet.show_stale = !self.quiet.show_stale;
        cx.notify();
    }
}
