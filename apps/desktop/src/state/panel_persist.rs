//! Collapsed side panels survive restarts: the open state is compared once per frame with
//! `settings.hidden_panels` and saved when it changed, whichever toggle, drag or command changed it.

use gpui::Context;

use crate::app::GitViewer;
use crate::settings::HiddenPanels;

/// Hidden flags from the live open state.
pub(crate) fn hidden_from(
    sidebar_hidden: bool,
    preview_hidden: bool,
    tree_open: bool,
    right_open: bool,
) -> HiddenPanels {
    HiddenPanels {
        sidebar: sidebar_hidden,
        preview: preview_hidden,
        tree: !tree_open,
        right: !right_open,
    }
}

impl GitViewer {
    pub(crate) fn persist_panels(&mut self, cx: &mut Context<Self>) {
        let now = hidden_from(
            self.nav.sidebar_hidden,
            self.preview_hidden,
            self.review.tree_open,
            self.review.right_open,
        );
        if now != self.settings.hidden_panels {
            self.settings.hidden_panels = now;
            self.save_settings(cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_state_maps_to_hidden_flags() {
        assert_eq!(
            hidden_from(false, false, true, true),
            HiddenPanels::default()
        );
        let h = hidden_from(true, true, false, false);
        assert!(h.sidebar && h.preview && h.tree && h.right);
    }

    #[test]
    fn legacy_settings_default_to_all_open() {
        let h: HiddenPanels = serde_json::from_str("{\"tree\":true}").unwrap();
        assert!(h.tree && !h.sidebar && !h.preview && !h.right);
    }
}
