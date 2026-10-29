//! Keyboard selection, persisted as a highlight.

use gpui::Context;
use gv_engine::ChangeRequestId;

use crate::app::GitViewer;

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
        self.settings.last_selected.as_deref().and_then(ChangeRequestId::parse_key)
    }
}
