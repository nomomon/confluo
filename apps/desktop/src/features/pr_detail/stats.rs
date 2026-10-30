//! `+a -d · N files` in muted mono. While counts load the +/- slot holds a pulsing placeholder of
//! about the same width, so the line never jumps or sits empty.

use gpui::{div, prelude::*};

use crate::ui::atoms::Skeleton;
use crate::ui::theme;

/// "1 file" / "N files".
pub(super) fn files_label(n: u32) -> String {
    if n == 1 {
        "1 file".to_string()
    } else {
        format!("{n} files")
    }
}

pub(super) fn stats(s: &gv_engine::ChangeRequest, loading: bool) -> impl IntoElement {
    let files = s.changed_files.map(files_label);
    let pending = loading && s.additions.is_none();
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(theme::space_1p5())
        .text_size(theme::text_xs())
        .font_family(theme::mono_font())
        .when(pending, |e| {
            e.child(Skeleton::new().w(theme::space_12()).h(theme::space_3()))
        })
        .when_some(s.additions, |e, a| {
            e.child(div().text_color(theme::success()).child(format!("+{a}")))
        })
        .when_some(s.deletions, |e, d| {
            e.child(div().text_color(theme::danger()).child(format!("-{d}")))
        })
        .when_some(files, |e, f| {
            e.when(s.additions.is_some() || pending, |e| {
                e.child(div().text_color(theme::text_subtle()).child("\u{00b7}"))
            })
            .child(div().text_color(theme::text_muted()).child(f))
        })
}

#[cfg(test)]
mod tests {
    use super::files_label;

    #[test]
    fn file_labels() {
        assert_eq!(files_label(1), "1 file");
        assert_eq!(files_label(3), "3 files");
    }
}
