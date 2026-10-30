//! One diff line: horizontal scroll, the hover "+" comment button (click, or drag across lines and release
//! to comment on the range), a dot on commented lines,
//! mouse line selection (click, Shift+click, drag) and the selection toolbar. Side by side, a row
//! holds two cells: the old side (deletions, comments go left) and the new side (additions, right).

use std::sync::Arc;

use gpui::{div, prelude::*, px, AnyElement, Context, MouseButton, MouseDownEvent, MouseMoveEvent, ScrollWheelEvent, Stateful, Div};
use gv_engine::{ChangedFile, LineKind};

use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::code_rows::{line_anchor, line_threads};
use crate::state::DetailState;
use crate::ui::atoms::{tooltip, Icon, IconName};
use crate::ui::molecules::code::{DiffRow, Lang, LineMark};
use crate::ui::theme;

const LINE_GROUP: &str = "diff-line";

impl GitViewer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_diff_line(&self, ix: usize, fi: usize, hi: usize, li: usize, files: &Arc<Vec<ChangedFile>>, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let bar = self.line_bar(fi, hi, &[li], d, cx);
        self.scrollable(self.card_side().id(("diff-line", ix)), fi, files, cx).child(self.diff_cell(ix, 0, fi, hi, li, files, d, false, cx)).children(bar).into_any_element()
    }

    /// Side-by-side row: old side cell, a divider, new side cell; missing sides are blank.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_pair(&self, ix: usize, fi: usize, hi: usize, a: Option<usize>, b: Option<usize>, files: &Arc<Vec<ChangedFile>>, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let lines: Vec<usize> = a.into_iter().chain(b).collect();
        let bar = self.line_bar(fi, hi, &lines, d, cx);
        let blank = || div().flex_1().min_w(theme::SPACE_0).h(layout::DIFF_LINE_H).bg(theme::surface());
        let cell = |side: usize, li: Option<usize>, cx: &mut Context<Self>| match li {
            Some(li) => div().flex_1().min_w(theme::SPACE_0).child(self.diff_cell(ix, side, fi, hi, li, files, d, true, cx)).into_any_element(),
            None => blank().into_any_element(),
        };
        let left = cell(1, a, cx);
        let right = cell(2, b, cx);
        self.scrollable(self.card_side().id(("diff-pair", ix)), fi, files, cx)
            .flex()
            .child(left)
            .child(div().w(px(1.)).h(layout::DIFF_LINE_H).bg(theme::border_subtle()))
            .child(right)
            .children(bar)
            .into_any_element()
    }

    /// The selection toolbar hangs under the last selected line.
    fn line_bar(&self, fi: usize, hi: usize, lines: &[usize], d: &DetailState, cx: &mut Context<Self>) -> Option<AnyElement> {
        d.code.sel.filter(|s| s.fi == fi && s.hi == hi && lines.contains(&s.range.hi()) && !d.code.dragging).map(|_| self.selection_bar(true, !d.code.sel_kbd, cx).into_any_element())
    }

    /// Horizontal wheel scrolls the file's code (shared by both sides).
    fn scrollable(&self, row: Stateful<Div>, fi: usize, files: &Arc<Vec<ChangedFile>>, cx: &mut Context<Self>) -> Stateful<Div> {
        let files = files.clone();
        row.relative().on_scroll_wheel(cx.listener(move |this, ev: &ScrollWheelEvent, _, cx| {
            let delta = ev.delta.pixel_delta(layout::DIFF_LINE_H);
            if delta.x.abs() <= delta.y.abs() {
                return;
            }
            cx.stop_propagation();
            let max_chars = files.get(fi).into_iter().flat_map(|f| &f.hunks).flat_map(|h| &h.lines).map(|l| l.text.len()).max().unwrap_or(0);
            let max_x = (theme::text_xs() * layout::MONO_CHAR_RATIO * max_chars as f32 - layout::DIFF_GUTTER_W * 4.).max(px(0.));
            if let Some(d) = this.detail.as_mut() {
                let e = d.code.scroll_x.entry(fi).or_insert(px(0.));
                *e = (*e - delta.x).clamp(px(0.), max_x);
            }
            cx.notify()
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn diff_cell(&self, ix: usize, side: usize, fi: usize, hi: usize, li: usize, files: &Arc<Vec<ChangedFile>>, d: &DetailState, split: bool, cx: &mut Context<Self>) -> AnyElement {
        let Some(f) = files.get(fi) else { return div().into_any_element() };
        let Some(l) = f.hunks.get(hi).and_then(|h| h.lines.get(li)) else { return div().into_any_element() };
        let mark = match l.kind {
            LineKind::Add => LineMark::Add,
            LineKind::Del => LineMark::Del,
            LineKind::Context => LineMark::Context,
        };
        let x = d.code.scroll_x.get(&fi).copied().unwrap_or(px(0.));
        let has_threads = line_threads(&d.code.inline.index, f.path(), l).next().is_some();
        let anchor = line_anchor(l);
        let in_sel = d.code.sel.is_some_and(|s| s.covers(fi, hi, li));
        let selected = in_sel || anchor.is_some_and(|(side, n)| d.code.inline.draft.as_ref().is_some_and(|dr| dr.covers(fi, side, n)));
        let group = gpui::SharedString::from(format!("{LINE_GROUP}-{side}"));
        let plus = anchor.map(|_| {
            div()
                .id(("add-line-comment", ix * 4 + side))
                .flex()
                .items_center()
                .justify_center()
                .size(theme::SPACE_4)
                .rounded(theme::RADIUS_SM)
                .bg(theme::accent())
                .opacity(0.)
                .group_hover(group.clone(), |s| s.opacity(1.))
                .cursor_pointer()
                .tooltip(tooltip("Comment (drag to select lines)"))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    window.focus(&this.focus);
                    this.press_line_plus(fi, hi, li, cx)
                }))
                .child(Icon::new(IconName::Plus).size(theme::ICON_SM).color(theme::bg()))
        });
        let (old_no, new_no) = match (split, side) {
            (true, 1) => (l.old_no, None),
            (true, _) => (None, l.new_no),
            _ => (l.old_no, l.new_no),
        };
        let mut row = DiffRow::new(mark, l.text.clone(), Lang::from_path(f.path())).numbers(old_no, new_no).gutter_w(layout::DIFF_GUTTER_W).height(layout::DIFF_LINE_H).scroll_x(x).dot(has_threads).selected(selected);
        if split {
            row = row.split_cell();
        }
        if let Some(p) = plus {
            row = row.action(p);
        }
        div()
            .id(("diff-cell", ix * 4 + side))
            .group(group)
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                window.focus(&this.focus);
                this.press_diff_line(fi, hi, li, ev.modifiers.shift, cx)
            }))
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, window, cx| match ev.pressed_button {
                Some(MouseButton::Left) => this.drag_diff_line(fi, hi, li, cx),
                _ => this.finish_line_drag(window, cx),
            }))
            .child(row)
            .into_any_element()
    }
}
