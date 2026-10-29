//! One row of the virtualized diff: file header, hunk band, line, notice or card end.
//!
//! Rows of a file share left and right borders so together they read as one rounded card.

use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context, Div, ScrollWheelEvent, SharedString};
use gv_engine::{ChangeRequest, FileStatus, LineKind, ProviderKind};

use super::{diff_counts, status_color};
use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::code_rows::{Notice, Row};
use crate::state::Copied;
use crate::ui::atoms::{Button, ButtonVariant, Icon, IconName};
use crate::ui::molecules::code::{DiffRow, Lang, LineMark};
use crate::ui::theme;

/// The provider's own diff page for files we cannot render.
fn files_url(c: &ChangeRequest) -> String {
    match c.id.repo.kind {
        ProviderKind::GitLab => format!("{}/diffs", c.web_url),
        _ => format!("{}/files", c.web_url),
    }
}

impl GitViewer {
    fn card_side(&self) -> Div {
        div().w_full().bg(self.surfaces().card()).border_l_1().border_r_1().border_color(theme::border_subtle()).overflow_hidden()
    }

    pub(super) fn render_code_row(&mut self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(d) = self.detail.as_ref() else { return div().into_any_element() };
        let (Some(files), Some(&row)) = (d.code.files.clone(), d.code.rows.get(ix)) else { return div().into_any_element() };
        let f = &files[row.file()];
        match row {
            Row::Header(fi) => self.render_file_header(fi, f, d.code.collapsed.contains(&fi), cx).into_any_element(),
            Row::Hunk(_, hi) => self
                .card_side()
                .h(layout::DIFF_HUNK_H)
                .flex()
                .items_center()
                .px(theme::SPACE_3)
                .bg(theme::diff_hunk_bg())
                .font_family(theme::mono_font())
                .text_size(theme::text_xs())
                .text_color(theme::text_subtle())
                .whitespace_nowrap()
                .child(f.hunks[hi].header.clone())
                .into_any_element(),
            Row::Line(fi, hi, li) => {
                let l = &f.hunks[hi].lines[li];
                let mark = match l.kind {
                    LineKind::Add => LineMark::Add,
                    LineKind::Del => LineMark::Del,
                    LineKind::Context => LineMark::Context,
                };
                let x = d.code.scroll_x.get(&fi).copied().unwrap_or(px(0.));
                let files_for_scroll = files.clone();
                self.card_side()
                    .id(("diff-line", ix))
                    .on_scroll_wheel(cx.listener(move |this, ev: &ScrollWheelEvent, _, cx| {
                        let delta = ev.delta.pixel_delta(layout::DIFF_LINE_H);
                        if delta.x.abs() <= delta.y.abs() {
                            return;
                        }
                        cx.stop_propagation();
                        let max_chars = files_for_scroll[fi].hunks.iter().flat_map(|h| &h.lines).map(|l| l.text.len()).max().unwrap_or(0);
                        let max_x = (theme::text_xs() * layout::MONO_CHAR_RATIO * max_chars as f32 - layout::DIFF_GUTTER_W * 4.).max(px(0.));
                        if let Some(d) = this.detail.as_mut() {
                            let e = d.code.scroll_x.entry(fi).or_insert(px(0.));
                            *e = (*e - delta.x).clamp(px(0.), max_x);
                        }
                        cx.notify()
                    }))
                    .child(
                        DiffRow::new(mark, l.text.clone(), Lang::from_path(f.path()))
                            .numbers(l.old_no, l.new_no)
                            .gutter_w(layout::DIFF_GUTTER_W)
                            .height(layout::DIFF_LINE_H)
                            .scroll_x(x),
                    )
                    .into_any_element()
            }
            Row::Notice(fi, n) => self.render_notice(fi, n, &d.summary, cx).into_any_element(),
            Row::End(_) => div()
                .w_full()
                .pb(theme::SPACE_3)
                .child(div().w_full().h(theme::SPACE_1P5).bg(self.surfaces().card()).rounded_b(theme::RADIUS_LG).border_1().border_t_0().border_color(theme::border_subtle()))
                .into_any_element(),
        }
    }

    fn render_notice(&self, fi: usize, n: Notice, cr: &ChangeRequest, cx: &mut Context<Self>) -> impl IntoElement {
        let url = files_url(cr);
        let (text, action): (String, Button) = match n {
            Notice::Binary => ("Binary file not shown".into(), Button::new(format!("open-bin-{fi}"), "Open in browser")),
            Notice::TooLarge => ("Diff too large to show here".into(), Button::new(format!("open-big-{fi}"), "Open in browser")),
            Notice::Large(lines) => (format!("Large diff, {lines} lines"), Button::new(format!("show-diff-{fi}"), "Show diff")),
            Notice::Empty => ("No content changes".into(), Button::new(format!("open-empty-{fi}"), "Open in browser")),
        };
        let action = if let Notice::Large(_) = n {
            action.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.code.show_large(fi);
                }
                cx.notify()
            }))
        } else {
            action.icon(IconName::ExternalLink).on_click(move |_, _, cx| cx.open_url(&url))
        };
        self.card_side()
            .flex()
            .items_center()
            .gap(theme::SPACE_3)
            .px(theme::SPACE_4)
            .py(theme::SPACE_3)
            .text_size(theme::text_sm())
            .text_color(theme::text_muted())
            .child(text)
            .child(action)
    }

    fn render_file_header(&self, fi: usize, f: &gv_engine::ChangedFile, collapsed: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let path = f.path().to_string();
        let name: SharedString = if f.status == FileStatus::Renamed { format!("{} -> {}", f.old_path, f.new_path).into() } else { path.clone().into() };
        let copy_path = path.clone();
        let view_path = path.clone();
        div()
            .id(("file-head", fi))
            .w_full()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .h(layout::DIFF_HEADER_H)
            .px(theme::SPACE_3)
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
            .child(Icon::new(if collapsed { IconName::ChevronRight } else { IconName::ChevronDown }).size(theme::ICON_SM))
            .child(Icon::new(IconName::FileCode).color(status_color(f.status)))
            .child(
                div()
                    .flex_1()
                    .min_w(theme::SPACE_0)
                    .truncate()
                    .font_family(theme::mono_font())
                    .text_size(theme::text_xs())
                    .text_color(if f.status == FileStatus::Deleted { theme::text_muted() } else { theme::text() })
                    .child(name),
            )
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
            .when(f.status != FileStatus::Deleted && !f.binary, |el| {
                el.child(Button::new(format!("view-file-{fi}"), "View file").variant(ButtonVariant::Ghost).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    this.open_file_view(view_path.clone(), cx)
                })))
            })
    }
}
