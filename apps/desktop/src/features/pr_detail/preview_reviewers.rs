//! Triage preview: the Reviewers line under the stats and CI line. An avatar group with each
//! reviewer's state (hover lists them), then the approvals summary, muted.

use gpui::{div, prelude::*};

use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::theme;

impl GitViewer {
    /// None when the PR has no reviewers and no approval rule.
    pub(super) fn preview_reviewers(&self, d: &DetailState) -> Option<gpui::Div> {
        let s = &d.summary;
        let approvals = self.approvals(d, true);
        if s.reviewers.is_empty() && approvals.is_none() {
            return None;
        }
        let users: Vec<_> = s.reviewers.iter().map(|r| r.user.clone()).collect();
        let states: Vec<_> = s.reviewers.iter().map(|r| r.state).collect();
        Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x(theme::space_3())
                .gap_y(theme::space_1())
                .text_size(theme::text_xs())
                .text_color(theme::text_muted())
                .child(div().text_color(theme::text_subtle()).child("Reviewers"))
                .when(!users.is_empty(), |e| {
                    let panel = self.surfaces().panel();
                    e.child(
                        self.people_group("preview-reviewers", &users, Some(&states))
                            .ring_over(panel, panel),
                    )
                })
                .children(approvals),
        )
    }
}
