//! Shortcuts: read-only list of key bindings.

use gpui::{div, prelude::*};

use crate::app::GitViewer;
use crate::ui::atoms::Kbd;
use crate::ui::molecules::Field;

const GROUPS: [(&str, &[(&str, &str)]); 3] = [
    ("General", &[("Settings", "⌘ ,"), ("Refresh", "⌘ R"), ("Search", "/"), ("Close or clear", "esc")]),
    ("List", &[("Next / previous", "J K"), ("Open", "↩"), ("Open in browser", "O"), ("Copy checkout", "C"), ("Star", "S"), ("Authored / Reviewing / All", "1 2 3")]),
    ("Code tab", &[("Next / previous file", "N P"), ("Collapse file", "X"), ("Move line cursor", "↑ ↓"), ("Extend selection", "⇧ ↑"), ("Comment on lines", "C"), ("Copy lines", "⌘ C")]),
];

impl GitViewer {
    pub(super) fn render_shortcuts(&self) -> impl IntoElement {
        div().children(GROUPS.iter().map(|(title, rows)| {
            self.settings_group(Some(*title), rows.iter().map(|(label, keys)| Field::new(*label, Kbd::new(*keys)).horizontal().into_any_element()).collect())
        }))
    }
}
