//! Overview card: status, review, branch and diff size.

use gpui::{div, prelude::*};
use gv_engine::ChangeRequest;

use crate::features::shared::{review_summary, state_pill};

use crate::ui::molecules::{Card, KeyValueRow};
use crate::ui::theme::{self, Translucency};

pub(super) fn overview_card(s: &ChangeRequest, surfaces: Translucency) -> impl IntoElement {
    let files = match s.changed_files {
        Some(1) => "1 file".to_string(),
        Some(n) => format!("{n} files"),
        None => "-".into(),
    };
    // GitLab lists carry no line counts; show +/- only once they are known.
    let lines = s.additions.zip(s.deletions).map(|(a, d)| {
        div().flex().gap(theme::SPACE_1).child(div().text_color(theme::success()).child(format!("+{a}"))).child(div().text_color(theme::danger()).child(format!("-{d}")))
    });
    let branch = format!("{} -> {}", s.source_branch, s.target_branch);
    Card::new(surfaces)
        .rows()
        .child(KeyValueRow::new("Status", state_pill(s.state)))
        .child(KeyValueRow::new("Review", review_summary(s)))
        .child(KeyValueRow::new("Branch", div().min_w(theme::SPACE_0).truncate().text_size(theme::text_sm()).font_family(theme::mono_font()).child(branch)))
        .child(KeyValueRow::new("Changes", div().flex().gap(theme::SPACE_2).child(files).children(lines)))
}
