//! Find in diff and go to line, wired to the app: open and close the bars, recompute matches, and
//! reveal the current match (expanding its file, scrolling the list and the code sideways).

use gpui::{px, Context, Focusable, ScrollStrategy, Window};

use super::find::{find_matches, first_from};
use super::DetailTab;
use crate::app::GitViewer;
use crate::features::layout;
use crate::ui::theme;

impl GitViewer {
    /// Cmd+F in Review mode: show the find bar on the diff with the query selected.
    pub(crate) fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .detail
            .as_ref()
            .is_some_and(|d| d.tab != DetailTab::Code)
        {
            self.set_detail_tab(DetailTab::Code, cx);
        }
        if let Some(d) = self.detail.as_mut() {
            d.code.viewer = None;
        }
        self.find.open = true;
        let input = self.find_input.clone();
        input.update(cx, |i, cx| i.select_all_text(cx));
        window.focus(&input.focus_handle(cx));
        self.run_find(cx);
    }

    pub(crate) fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find.open = false;
        window.focus(&self.focus);
        cx.notify();
    }

    /// Recompute matches for the query; the first one at or after the file in view becomes current.
    pub(crate) fn run_find(&mut self, cx: &mut Context<Self>) {
        let q = self.find_input.read(cx).text();
        let Some(d) = self.detail.as_ref() else {
            return;
        };
        let files = d.code.files.clone().unwrap_or_default();
        self.find.matches = find_matches(&files, &q, self.find.case);
        self.find.current = first_from(&self.find.matches, d.code.current_file());
        self.reveal_match(cx);
    }

    pub(crate) fn toggle_find_case(&mut self, cx: &mut Context<Self>) {
        self.find.case = !self.find.case;
        self.run_find(cx);
    }

    /// Enter / Shift+Enter and the arrows: next or previous match, wrapping.
    pub(crate) fn find_step(&mut self, delta: isize, cx: &mut Context<Self>) {
        self.find.step(delta);
        self.reveal_match(cx);
    }

    /// Expand the current match's file if needed, scroll its row into view and its column on screen.
    fn reveal_match(&mut self, cx: &mut Context<Self>) {
        let wrap = self.settings.diff_wrap;
        let Some(m) = self.find.matches.get(self.find.current).cloned() else {
            cx.notify();
            return;
        };
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        let c = &mut d.code;
        let large = c
            .files
            .as_ref()
            .and_then(|f| f.get(m.fi))
            .is_some_and(|f| f.line_count() > super::code_rows::LARGE_FILE_LINES);
        let expanded = c.collapsed.remove(&m.fi) | (large && c.shown_large.insert(m.fi));
        if expanded {
            c.rebuild();
        }
        let start = c.starts.get(m.fi).copied().unwrap_or(0);
        if let Some(ix) = c.rows[start..]
            .iter()
            .position(|r| r.has_line(m.fi, m.hi, m.li))
        {
            if expanded {
                // Freshly reset rows are unmeasured; place the line a few rows below the top.
                c.list.scroll_to(gpui::ListOffset {
                    item_ix: (start + ix).saturating_sub(3),
                    offset_in_item: px(0.),
                });
            } else {
                c.list.scroll_to_reveal_item(start + ix);
            }
        }
        if !wrap {
            let char_w = theme::text_xs() * layout::MONO_CHAR_RATIO;
            let col = char_w * m.range.start as f32;
            let view = c.list.viewport_bounds().size.width - layout::DIFF_GUTTER_W * 3.;
            let cur = c.scroll_x.get(&m.fi).copied().unwrap_or(px(0.));
            if col < cur || col + char_w * m.range.len() as f32 > cur + view {
                c.scroll_x.insert(m.fi, (col - view / 3.).max(px(0.)));
            }
        }
        cx.notify();
    }

    /// Ctrl+G in the file viewer: show the line number field.
    pub(crate) fn open_goto_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(v) = self.detail.as_mut().and_then(|d| d.code.viewer.as_mut()) else {
            self.show_info_toast("Go to line works in the file viewer (View file)", cx);
            return;
        };
        v.goto = true;
        self.goto_input.update(cx, |i, cx| i.set_text("", cx));
        window.focus(&self.goto_input.focus_handle(cx));
        cx.notify();
    }

    /// Enter in the line field: select that line and scroll it to the middle.
    pub(crate) fn submit_goto_line(&mut self, cx: &mut Context<Self>) {
        let text = self.goto_input.read(cx).text();
        let Some(v) = self.detail.as_mut().and_then(|d| d.code.viewer.as_mut()) else {
            return;
        };
        v.goto = false;
        let count = v.lines.as_ref().map_or(0, |l| l.len());
        if let Some(i) = goto_index(&text, count) {
            v.sel = Some(super::line_select::LineRange::at(i));
            v.scroll.scroll_to_item(i, ScrollStrategy::Center);
        }
        self.refocus = true;
        cx.notify();
    }
}

/// 0-based line for the typed 1-based number, clamped to the file; `None` when not a number.
pub fn goto_index(text: &str, count: usize) -> Option<usize> {
    let n: usize = text.trim().trim_start_matches(':').parse().ok()?;
    (count > 0).then(|| n.clamp(1, count) - 1)
}

#[cfg(test)]
mod tests {
    #[test]
    fn goto_line_parses_and_clamps() {
        use super::goto_index;
        assert_eq!(goto_index("12", 100), Some(11));
        assert_eq!(goto_index(":3", 100), Some(2));
        assert_eq!(goto_index("0", 100), Some(0));
        assert_eq!(goto_index("500", 100), Some(99));
        assert_eq!(goto_index("x", 100), None);
        assert_eq!(goto_index("5", 0), None);
    }
}
