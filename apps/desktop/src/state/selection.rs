//! Keyboard selection: persisted as a highlight and kept scrolled into view.

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

    /// The highlight restored from settings; never opens the detail pane.
    pub(crate) fn restored_cursor(&self) -> Option<ChangeRequestId> {
        self.settings
            .last_selected
            .as_deref()
            .and_then(ChangeRequestId::parse_key)
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
