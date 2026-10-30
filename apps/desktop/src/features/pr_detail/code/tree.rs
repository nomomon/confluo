//! Review mode file tree: viewed progress, filter, "Hide viewed", foldable folders, and per file
//! a viewed checkbox, status color, comment count and +/- counts. Clicking a file scrolls to it.

use gpui::{div, prelude::*, ClickEvent, Context};

use super::{comment_count, diff_counts, status_color};
use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::{tree_entries, DetailState, TreeEntry, TreeQuery};
use crate::ui::atoms::{tooltip, Checkbox, Icon, IconName, ListItem, Switch};
use crate::ui::molecules::command_palette::score;
use crate::ui::molecules::Input;
use crate::ui::theme;

impl GitViewer {
    pub(in crate::features::pr_detail) fn render_file_tree(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let files = d.code.files.clone().unwrap_or_default();
        let r = &self.review;
        let current = d.code.current_file();
        let total = files.len();
        let seen = files.iter().filter(|f| r.viewed.contains(f.path())).count();
        let filter = self.tree_filter.read(cx).text();
        let paths: Vec<&str> = files.iter().map(|f| f.path()).collect();
        let entries = tree_entries(&paths, &TreeQuery { filter: &filter, hide_viewed: r.hide_viewed, viewed: &r.viewed, folded: &r.folded }, score);
        let frac = if total == 0 { 0. } else { seen as f32 / total as f32 };
        let progress = div().w_full().h(theme::SPACE_0P5).bg(theme::border_subtle()).child(div().h_full().w(gpui::relative(frac)).bg(theme::success()));
        let head = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .px(theme::SPACE_3)
            .pt(theme::SPACE_2)
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
            .child(div().flex_1().min_w(theme::SPACE_0).overflow_hidden().flex().items_center().gap(theme::SPACE_2).child(div().flex_shrink_0().child(format!("{seen} of {total} viewed"))).child(
                div()
                    .flex()
                    .gap(theme::SPACE_1)
                    .font_family(theme::mono_font())
                    .when_some(d.summary.additions, |e, a| e.child(div().text_color(theme::success()).child(format!("+{a}"))))
                    .when_some(d.summary.deletions, |e, n| e.child(div().text_color(theme::danger()).child(format!("-{n}")))),
            ))
            .child(div().flex_shrink_0().text_color(theme::text_subtle()).child("Hide viewed"))
            .child(Switch::new("hide-viewed", r.hide_viewed).on_change(cx.listener(|this, on: &bool, _, cx| {
                this.review.hide_viewed = *on;
                cx.notify()
            })));
        let input = Input::new("tree-filter", self.tree_filter.clone(), !filter.is_empty()).leading(IconName::Search).on_clear(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.tree_filter.update(cx, |i, cx| i.set_text("", cx));
        }));
        let mut col = crate::ui::elastic::content("file-tree").flex().flex_col().gap(theme::SPACE_0P5);
        for e in entries {
            col = col.child(match e {
                TreeEntry::Dir(dir, n) => {
                    let folded = r.folded.contains(&dir);
                    let key = dir.clone();
                    div()
                        .id(gpui::SharedString::from(format!("tree-dir-{dir}")))
                        .flex()
                        .items_center()
                        .gap(theme::SPACE_1)
                        .mt(theme::SPACE_1)
                        .px(theme::SPACE_1)
                        .py(theme::SPACE_0P5)
                        .list_item_radius(false, theme::RADIUS_SM)
                        .text_size(theme::text_xs())
                        .text_color(theme::text_subtle())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            if !this.review.folded.remove(&key) {
                                this.review.folded.insert(key.clone());
                            }
                            cx.notify()
                        }))
                        .child(Icon::new(if folded { IconName::ChevronRight } else { IconName::ChevronDown }).size(theme::ICON_SM).color(theme::text_subtle()))
                        .child(Icon::new(IconName::Folder).size(theme::ICON_SM).color(theme::text_subtle()))
                        .child(div().flex_1().min_w(theme::SPACE_0).truncate().child(dir.clone()))
                        .when(folded, |el| el.child(n.to_string()))
                        .into_any_element()
                }
                TreeEntry::File(fi) => match files.get(fi) { Some(f) => self.tree_file(fi, f, fi == current, !filter.is_empty(), d, cx).into_any_element(), None => div().into_any_element() },
            });
        }
        div().flex().flex_col().size_full().child(progress).child(head).child(div().px(theme::SPACE_2).py(theme::SPACE_2).child(input)).child({
            let h = crate::ui::elastic::handle("file-tree");
            let pane = div().id("file-tree").track_scroll(&h).map(crate::ui::scroll::y_only).flex_1().min_h(theme::SPACE_0).px(theme::SPACE_2).pb(layout::END_PAD).child(col);
            crate::ui::elastic::y_scroller(pane, "file-tree", h)
        })
    }

    fn tree_file(&self, fi: usize, f: &gv_engine::ChangedFile, current: bool, full_path: bool, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let path = f.path();
        let name = if full_path { path.to_string() } else { path.rsplit_once('/').map(|(_, n)| n).unwrap_or(path).to_string() };
        let viewed = self.review.viewed.contains(path);
        div()
            .id(("tree-file", fi))
            .flex()
            .items_center()
            .gap(theme::SPACE_1P5)
            .pl(if full_path { theme::SPACE_1 } else { theme::SPACE_5 })
            .pr(theme::SPACE_1)
            .py(theme::SPACE_1)
            .list_item_radius(current, theme::RADIUS_SM)
            .tooltip(tooltip(path.to_string()))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    if d.code.collapsed.remove(&fi) {
                        d.code.rebuild();
                    }
                    d.code.goto(fi);
                }
                cx.notify()
            }))
            .child(div().on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation()).child(
                Checkbox::new(format!("viewed-{fi}"), viewed).on_change(cx.listener(move |this, _: &bool, _, cx| this.toggle_viewed(fi, cx))),
            ))
            .child(
                div()
                    .flex_1()
                    .min_w(theme::SPACE_0)
                    .truncate()
                    .text_size(theme::text_xs())
                    .text_color(if viewed { theme::text_subtle() } else { status_color(f.status) })
                    .child(name),
            )
            .children(needs_me(path, d).then(|| div().flex_shrink_0().size(theme::SPACE_1P5).rounded(theme::RADIUS_FULL).bg(theme::warning())))
            .children(comment_count(d.code.inline.index.count(path)))
            .child(diff_counts(f.additions, f.deletions))
    }
}

/// A thread on this file waits for me (amber dot in the tree).
fn needs_me(path: &str, d: &DetailState) -> bool {
    let Some(det) = d.detail.as_ref() else { return false };
    if !det.threads.iter().any(|t| t.position.as_ref().is_some_and(|p| p.path == path)) {
        return false;
    }
    det.threads.iter().zip(crate::state::attention(d)).any(|(t, a)| a == gv_engine::Attention::NeedsMe && t.position.as_ref().is_some_and(|p| p.path == path))
}
