//! One row of the virtualized diff: file header, hunk band, line, notice or card end.
//!
//! Rows of a file share left and right borders so together they read as one rounded card.

use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context, Div, SharedString};
use gv_engine::{ChangeRequest, FileStatus, ProviderKind};

use super::{comment_count, diff_counts, status_color};
use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::code_rows::{Notice, Row};
use crate::state::Copied;
use crate::ui::atoms::{Button, ButtonVariant, Icon, IconName};
use crate::ui::theme;

/// The provider's own diff page for files we cannot render.
fn files_url(c: &ChangeRequest) -> String {
    match c.id.repo.kind {
        ProviderKind::GitLab => format!("{}/diffs", c.web_url),
        _ => format!("{}/files", c.web_url),
    }
}

impl GitViewer {
    /// "Viewed" checkbox in the file card header (v).
    fn viewed_toggle(&self, fi: usize, path: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let on = self.review.viewed.contains(path);
        div()
            .id(("file-viewed", fi))
            .flex()
            .items_center()
            .gap(theme::space_1())
            .px(theme::space_1p5())
            .py(theme::space_0p5())
            .rounded(theme::RADIUS_SM)
            .border_1()
            .border_color(theme::border_subtle())
            .when(on, |e| e.bg(theme::surface_active()))
            .text_size(theme::text_xs())
            .text_color(if on {
                theme::text()
            } else {
                theme::text_muted()
            })
            .tooltip(crate::ui::atoms::tooltip(crate::features::keymap::tip::<
                crate::features::keymap::ToggleViewed,
            >("Mark as viewed")))
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.toggle_viewed(fi, cx)
            }))
            .child(
                Icon::new(if on { IconName::Check } else { IconName::Eye })
                    .size(theme::icon_sm())
                    .color(if on {
                        theme::success()
                    } else {
                        theme::text_subtle()
                    }),
            )
            .child("Viewed")
    }

    pub(super) fn card_side(&self) -> Div {
        div()
            .w_full()
            .bg(self.surfaces().card())
            .border_l_1()
            .border_r_1()
            .border_color(theme::border_subtle())
            .overflow_hidden()
    }

    pub(super) fn render_code_row(
        &mut self,
        ix: usize,
        view_h: gpui::Pixels,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(d) = self.detail.as_ref() else {
            return div().into_any_element();
        };
        if ix == d.code.rows.len() {
            let h = if view_h > px(0.) {
                view_h * layout::SCROLL_PAST_END
            } else {
                layout::SCROLL_PAST_END_FALLBACK
            };
            return div().h(h).into_any_element();
        }
        let (Some(files), Some(&row)) = (d.code.files.clone(), d.code.rows.get(ix)) else {
            return div().into_any_element();
        };
        let Some(f) = files.get(row.file()) else {
            return div().into_any_element();
        };
        match row {
            Row::Header(fi) => self
                .render_file_header(
                    fi,
                    f,
                    d.code.collapsed.contains(&fi),
                    d.code.inline.index.count(f.path()),
                    cx,
                )
                .into_any_element(),
            Row::Hunk(_, hi) => self
                .card_side()
                .h(layout::DIFF_HUNK_H)
                .flex()
                .items_center()
                .px(theme::space_3())
                .bg(theme::diff_hunk_bg())
                .font_family(theme::mono_font())
                .text_size(theme::text_xs())
                .text_color(theme::text_subtle())
                .whitespace_nowrap()
                .child(
                    f.hunks
                        .get(hi)
                        .map(|h| h.header.clone())
                        .unwrap_or_default(),
                )
                .into_any_element(),
            Row::Line(fi, hi, li) => self.render_diff_line(ix, fi, hi, li, &files, d, cx),
            Row::Pair(fi, hi, a, b) => self.render_pair(ix, fi, hi, a, b, &files, d, cx),
            Row::Thread(fi, ti) => self.render_inline_thread(fi, ti, d, cx),
            Row::Composer(_) => self.render_line_composer(d, cx),
            Row::Outdated(_, n) => self
                .card_side()
                .px(theme::space_3())
                .pt(theme::space_3())
                .pb(theme::space_1())
                .border_t_1()
                .text_size(theme::text_xs())
                .text_color(theme::text_subtle())
                .child(format!("Outdated comments ({n})"))
                .into_any_element(),
            Row::Notice(fi, n) => self.render_notice(fi, n, &d.summary, cx).into_any_element(),
            Row::End(_) => div()
                .w_full()
                .pb(theme::space_3())
                .child(
                    div()
                        .w_full()
                        .h(theme::space_1p5())
                        .bg(self.surfaces().card())
                        .rounded_b(theme::RADIUS_LG)
                        .border_1()
                        .border_t_0()
                        .border_color(theme::border_subtle()),
                )
                .into_any_element(),
        }
    }

    fn render_notice(
        &self,
        fi: usize,
        n: Notice,
        cr: &ChangeRequest,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let url = files_url(cr);
        let (text, action): (String, Button) = match n {
            Notice::Binary => (
                "Binary file not shown".into(),
                Button::new(format!("open-bin-{fi}"), "Open in browser"),
            ),
            Notice::TooLarge => (
                "Diff too large to show here".into(),
                Button::new(format!("open-big-{fi}"), "Open in browser"),
            ),
            Notice::Large(lines) => (
                format!("Large diff, {lines} lines"),
                Button::new(format!("show-diff-{fi}"), "Show diff"),
            ),
            Notice::Empty => (
                "No content changes".into(),
                Button::new(format!("open-empty-{fi}"), "Open in browser"),
            ),
        };
        let action = if let Notice::Large(_) = n {
            action.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.code.show_large(fi);
                }
                cx.notify()
            }))
        } else {
            action
                .icon(IconName::ExternalLink)
                .on_click(move |_, _, cx| cx.open_url(&url))
        };
        self.card_side()
            .flex()
            .items_center()
            .gap(theme::space_3())
            .px(theme::space_4())
            .py(theme::space_3())
            .text_size(theme::text_sm())
            .text_color(theme::text_muted())
            .child(text)
            .child(action)
    }

    fn render_file_header(
        &self,
        fi: usize,
        f: &gv_engine::ChangedFile,
        collapsed: bool,
        comments: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let path = f.path().to_string();
        let name: SharedString = if f.status == FileStatus::Renamed {
            format!("{} -> {}", f.old_path, f.new_path).into()
        } else {
            path.clone().into()
        };
        let copy_path = path.clone();
        let view_path = path.clone();
        div()
            .id(("file-head", fi))
            .w_full()
            .flex()
            .items_center()
            .gap(theme::space_2())
            .h(layout::DIFF_HEADER_H)
            .px(theme::space_3())
            .bg(self.surfaces().card())
            .border_1()
            .border_color(theme::border_subtle())
            .rounded_t(theme::RADIUS_LG)
            .when(collapsed, |el| el.border_b_0())
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.code.toggle_file(fi);
                }
                cx.notify()
            }))
            .child(
                Icon::new(if collapsed {
                    IconName::ChevronRight
                } else {
                    IconName::ChevronDown
                })
                .size(theme::icon_sm()),
            )
            .child(Icon::new(IconName::FileCode).color(status_color(f.status)))
            .child({
                // File name first, then its directory muted; the directory gives way so the name stays readable.
                let (dir, file) = match name.rfind('/') {
                    Some(i) if f.status != FileStatus::Renamed => {
                        (name[..=i].to_string(), name[i + 1..].to_string())
                    }
                    _ => (String::new(), name.to_string()),
                };
                div()
                    .flex_1()
                    .min_w(theme::space_0())
                    .flex()
                    .gap(theme::space_2())
                    .overflow_hidden()
                    .font_family(theme::mono_font())
                    .text_size(theme::text_xs())
                    .child(
                        div()
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .text_color(if f.status == FileStatus::Deleted {
                                theme::text_muted()
                            } else {
                                theme::text()
                            })
                            .child(file),
                    )
                    .child(
                        div()
                            .min_w(theme::space_0())
                            .flex_shrink()
                            .truncate()
                            .text_color(theme::text_subtle())
                            .child(dir.trim_end_matches('/').to_string()),
                    )
            })
            .children(comment_count(comments))
            .child(diff_counts(f.additions, f.deletions))
            .child(
                Button::new(format!("copy-path-{fi}"), "")
                    .icon(IconName::Copy)
                    .variant(ButtonVariant::Ghost)
                    .tooltip("Copy path")
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.copy(Copied::Path, copy_path.clone(), cx)
                    })),
            )
            .child(self.viewed_toggle(fi, f.path(), cx))
            .when(f.status != FileStatus::Deleted && !f.binary, |el| {
                el.child(
                    Button::new(format!("view-file-{fi}"), "View file")
                        .variant(ButtonVariant::Ghost)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            this.open_file_view(view_path.clone(), cx)
                        })),
                )
            })
    }
}
