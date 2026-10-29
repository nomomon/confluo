//! Overview card: status, review, branch and diff size.

use gpui::{div, prelude::*};
use gv_engine::ChangeRequest;

use crate::features::shared::{review_summary, state_pill};
use crate::ui::molecules::{Card, KeyValueRow};
use crate::ui::theme;

pub(super) fn overview_card(s: &ChangeRequest, opacity: f32) -> impl IntoElement {
    let changes = match (s.changed_files, s.additions, s.deletions) {
        (None, None, None) => "-".to_string(),
        (f, a, dl) => format!("{} files  +{}  -{}", f.unwrap_or(0), a.unwrap_or(0), dl.unwrap_or(0)),
    };
    let branch = format!("{} -> {}", s.source_branch, s.target_branch);
    Card::new(opacity)
        .child(KeyValueRow::new("Status", state_pill(s.state)))
        .child(KeyValueRow::new("Review", review_summary(s)))
        .child(KeyValueRow::new("Branch", div().text_xs().font_family(theme::MONO_FONT).child(branch)))
        .child(KeyValueRow::new("Changes", div().text_xs().child(changes)))
}
