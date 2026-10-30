//! Code tab file tree: paths grouped by folder with status colors and +/- counts.

use gpui::{div, prelude::*, ClickEvent, Context};

use super::{comment_count, diff_counts, status_color};
use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::DetailState;
use crate::ui::atoms::{Icon, IconName, ListItem};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_file_tree(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let files = d.code.files.clone().unwrap_or_default();
        let current = d.code.current_file();
        // Group by parent folder, keeping each folder's first appearance order sorted by path.
        let mut order: Vec<usize> = (0..files.len()).collect();
        order.sort_by(|a, b| files[*a].path().cmp(files[*b].path()));
        let mut col = div().id("file-tree").flex().flex_col().gap(theme::SPACE_0P5).overflow_y_scroll().size_full();
        let mut last_dir: Option<&str> = None;
        for fi in order {
            let f = &files[fi];
            let path = f.path();
            let (dir, name) = path.rsplit_once('/').unwrap_or(("", path));
            if last_dir != Some(dir) && !dir.is_empty() {
                col = col.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(theme::SPACE_1)
                        .pt(theme::SPACE_2)
                        .px(theme::SPACE_1)
                        .text_size(theme::text_xs())
                        .text_color(theme::text_subtle())
                        .child(Icon::new(IconName::Folder).size(theme::ICON_SM).color(theme::text_subtle()))
                        .child(div().truncate().child(dir.to_string())),
                );
            }
            last_dir = Some(dir);
            col = col.child(
                div()
                    .id(("tree-file", fi))
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_1)
                    .px(theme::SPACE_2)
                    .py(theme::SPACE_1)
                    .list_item_radius(fi == current, theme::RADIUS_SM)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if let Some(d) = this.detail.as_mut() {
                            d.code.goto(fi);
                        }
                        cx.notify()
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme::SPACE_0)
                            .truncate()
                            .text_size(theme::text_xs())
                            .text_color(status_color(f.status))
                            .child(name.to_string()),
                    )
                    .children(comment_count(d.code.inline.index.count(f.path())))
                    .child(diff_counts(f.additions, f.deletions)),
            );
        }
        div().flex_shrink_0().w(layout::FILE_TREE_W).h_full().child(col)
    }
}
