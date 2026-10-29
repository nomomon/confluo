//! One diff line row: horizontal scroll, the hover "+" comment button, a dot on commented lines,
//! mouse line selection (click, Shift+click, drag) and the selection toolbar.

use std::sync::Arc;

use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context, MouseButton, MouseDownEvent, MouseMoveEvent, ScrollWheelEvent};
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
        let f = &files[fi];
        let l = &f.hunks[hi].lines[li];
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
        let bar = d.code.sel.filter(|s| s.fi == fi && s.hi == hi && s.range.hi() == li && !d.code.dragging).map(|_| self.selection_bar(true, !d.code.sel_kbd, cx));
        let plus = anchor.map(|_| {
            div()
                .id(("add-line-comment", ix))
                .flex()
                .items_center()
                .justify_center()
                .size(theme::SPACE_4)
                .rounded(theme::RADIUS_SM)
                .bg(theme::accent())
                .opacity(0.)
                .group_hover(LINE_GROUP, |s| s.opacity(1.))
                .cursor_pointer()
                .tooltip(tooltip("Comment on this line"))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_line_draft(fi, hi, li, li, window, cx)
                }))
                .child(Icon::new(IconName::Plus).size(theme::ICON_SM).color(theme::bg()))
        });
        let files_for_scroll = files.clone();
        let mut row = DiffRow::new(mark, l.text.clone(), Lang::from_path(f.path()))
            .numbers(l.old_no, l.new_no)
            .gutter_w(layout::DIFF_GUTTER_W)
            .height(layout::DIFF_LINE_H)
            .scroll_x(x)
            .dot(has_threads)
            .selected(selected);
        if let Some(p) = plus {
            row = row.action(p);
        }
        self.card_side()
            .id(("diff-line", ix))
            .group(LINE_GROUP)
            .relative()
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                window.focus(&this.focus);
                this.press_diff_line(fi, hi, li, ev.modifiers.shift, cx)
            }))
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| match ev.pressed_button {
                Some(MouseButton::Left) => this.drag_diff_line(fi, hi, li, cx),
                _ => this.end_line_drag(cx),
            }))
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
            .child(row)
            .children(bar)
            .into_any_element()
    }
}
