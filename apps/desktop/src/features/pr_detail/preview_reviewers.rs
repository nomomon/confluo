//! Triage preview: the Reviewers line under the stats and CI line. Reviewers with their state (the
//! Overview's people list), then the approvals summary, muted. Wraps at narrow widths.

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
                .child(div().flex_1().min_w_0().child(self.people_value(
                    &users,
                    Some(&states),
                    false,
                )))
                .children(approvals),
        )
    }
}
