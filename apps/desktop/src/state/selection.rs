//! Selection: the previewed PR is the selected row; j/k move it (opening the preview) and keep it in view.

use gpui::{point, px, Bounds, Context, Pixels, ScrollHandle, Window};
use gv_engine::ChangeRequestId;

use crate::app::GitViewer;

/// Breathing room kept above or below a row revealed by j/k.
const REVEAL_PAD: Pixels = px(8.);

impl GitViewer {
    /// Move the highlight and remember it for the next launch.
    pub(crate) fn set_cursor(&mut self, id: ChangeRequestId, cx: &mut Context<Self>) {
        let key = id.key();
        self.cursor = Some(id);
        if self.settings.last_selected.as_deref() != Some(key.as_str()) {
            self.settings.last_selected = Some(key);
            self.save_settings(cx);
        }
    }

    /// The PR open in the visible preview: the only selected (lit) row.
    pub(crate) fn previewed(&self) -> Option<&ChangeRequestId> {
        previewed(
            self.preview_hidden,
            self.detail.as_ref().map(|d| &d.summary.id),
        )
    }
}

/// The selected row: the preview's PR while the preview shows, else none.
pub fn previewed(hidden: bool, open: Option<&ChangeRequestId>) -> Option<&ChangeRequestId> {
    open.filter(|_| !hidden)
}

/// Row index j/k (`delta`) select from `at` (the previewed row's index in the list, if it is in it): the
/// first (down) or last (up) row when nothing in this list is selected.
pub fn step_selection(at: Option<usize>, delta: isize, len: usize) -> usize {
    match at {
        Some(i) => (i as isize + delta).clamp(0, len as isize - 1) as usize,
        None if delta > 0 => 0,
        None => len - 1,
    }
}

/// Scroll `handle` the least amount that shows `row`. Returns true when it moved.
pub fn reveal(handle: &ScrollHandle, row: Bounds<Pixels>, _: &mut Window) -> bool {
    let view = handle.bounds();
    let offset = handle.offset();
    let max = handle.max_offset().height;
    let dy = if row.top() < view.top() {
        view.top() - row.top() + REVEAL_PAD
    } else if row.bottom() > view.bottom() {
        view.bottom() - row.bottom() - REVEAL_PAD
    } else {
        return false;
    };
    let y = (offset.y + dy).clamp(-max, px(0.));
    handle.set_offset(point(offset.x, y));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u64) -> ChangeRequestId {
        ChangeRequestId::parse_key(&format!("github:github.com/acme/web#{n}")).expect("key")
    }

    #[test]
    fn only_the_open_preview_is_selected() {
        let a = id(1);
        assert_eq!(previewed(false, Some(&a)), Some(&a));
        // Closing the preview clears the highlight.
        assert_eq!(previewed(true, Some(&a)), None);
        // Launch or a fresh list without a preview: nothing lit.
        assert_eq!(previewed(false, None), None);
    }

    #[test]
    fn first_key_in_a_fresh_list_selects_an_end() {
        // Entered a list (or switched tab) whose rows do not hold the previewed PR.
        assert_eq!(step_selection(None, 1, 5), 0);
        assert_eq!(step_selection(None, -1, 5), 4);
        // Afterwards keys move from the previewed row, clamped.
        assert_eq!(step_selection(Some(2), 1, 5), 3);
        assert_eq!(step_selection(Some(0), -1, 5), 0);
        assert_eq!(step_selection(Some(4), 1, 5), 4);
    }
}
