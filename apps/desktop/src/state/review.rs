//! Submitting reviews from the detail header with an optimistic state badge.

use gpui::{Context, Focusable};
use gv_engine::{state_after, ReviewAction};

use super::{bridge, err_str};
use crate::app::{GitViewer, Overlay};

impl GitViewer {
    /// Submit `action` on the open PR with the summary from the review dialog (if open).
    pub(crate) fn submit_review(&mut self, action: ReviewAction, cx: &mut Context<Self>) {
        let body = if matches!(self.overlay, Overlay::ReviewSummary(_)) {
            self.review_input.read(cx).text().trim().to_string()
        } else {
            String::new()
        };
        self.overlay = Overlay::None;
        self.layer = None;
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        let Some(det) = d.detail.as_mut() else { return };
        if !det.review.caps.allows(action) || d.posting {
            return;
        }
        let before = det.review.my_state;
        det.review.my_state = state_after(action, before);
        d.posting = true;
        let id = d.summary.id.clone();
        self.review_input.update(cx, |i, cx| i.set_text("", cx));
        let engine = self.engine.clone();
        let body_opt = (!body.is_empty()).then_some(body.clone());
        let rx = bridge::spawn(async move {
            engine
                .submit_review(&id, action, body_opt.as_deref())
                .await
                .map_err(err_str)
        });
        cx.spawn(async move |this, cx| {
            let r = rx.await.unwrap_or_else(|_| Err("cancelled".into()));
            this.update(cx, |v, cx| {
                let ok = r.is_ok();
                if let Some(d) = v.detail.as_mut() {
                    d.posting = false;
                    if let Err(e) = r {
                        d.error = Some(format!("{} failed: {e}", action.label()));
                        if let Some(det) = d.detail.as_mut() {
                            det.review.my_state = before;
                        }
                        if !body.is_empty() {
                            v.review_input
                                .update(cx, |i, cx| i.set_text(body.clone(), cx));
                        }
                    }
                }
                if ok {
                    v.show_toast(
                        match action {
                            ReviewAction::Approve => "Approved",
                            ReviewAction::RequestChanges => "Changes requested",
                            ReviewAction::Comment => "Review submitted",
                            ReviewAction::Unapprove => "Approval revoked",
                            ReviewAction::ApproveWithSuggestions => "Approved with suggestions",
                            ReviewAction::WaitForAuthor => "Waiting for the author",
                            ReviewAction::ResetVote => "Vote reset",
                        },
                        cx,
                    );
                }
                v.load_detail(cx);
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Open the summary dialog for `action` (Comment always asks; verdicts may add a note).
    pub(crate) fn open_review_summary(
        &mut self,
        action: ReviewAction,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        self.layer = None;
        self.overlay = Overlay::ReviewSummary(action);
        window.focus(&self.review_input.read(cx).focus_handle(cx));
        cx.notify();
    }
}
