//! Suggested changes: apply on the provider (GitLab) or copy the suggested text.

use gpui::Context;

use super::{bridge, err_str};
use crate::app::GitViewer;

impl GitViewer {
    /// Apply a GitLab suggestion (a commit on the source branch), then reload the detail.
    pub(crate) fn apply_suggestion(&mut self, suggestion_id: String, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        if d.code.inline.posting {
            return;
        }
        d.code.inline.posting = true;
        let id = d.summary.id.clone();
        let engine = self.engine.clone();
        let rx = bridge::spawn(async move {
            engine
                .apply_suggestion(&id, &suggestion_id)
                .await
                .map_err(err_str)
        });
        self.after_inline(rx, "Apply suggestion failed", cx);
        self.show_toast("Applying suggestion", cx);
    }
}
