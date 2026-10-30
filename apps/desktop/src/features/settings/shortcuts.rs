//! Shortcuts: read-only list of key bindings, straight from the keymap.

use gpui::{div, prelude::*};

use crate::app::GitViewer;
use crate::features::keymap::{display, GROUPS};
use crate::ui::atoms::Kbd;
use crate::ui::molecules::Field;

/// Code tab keys handled by the line cursor (not in the keymap table).
const LINE_KEYS: [(&str, &str); 4] = [("Move line cursor", "\u{2191} \u{2193}"), ("Extend selection", "\u{21e7} \u{2191}"), ("Comment on lines", "C"), ("Copy lines", "\u{2318} C")];

impl GitViewer {
    pub(super) fn render_shortcuts(&self) -> impl IntoElement {
        let groups = GROUPS.iter().map(|(title, rows)| {
            let mut fields: Vec<_> = rows
                .iter()
                .map(|e| {
                    let keys = e.keys.iter().map(|k| display(k)).collect::<Vec<_>>();
                    let row = div().flex().gap(gpui::px(6.)).children(keys.into_iter().map(Kbd::new));
                    Field::new(e.label, row).horizontal().into_any_element()
                })
                .collect();
            if *title == "Code tab" {
                fields.extend(LINE_KEYS.iter().map(|(l, k)| Field::new(*l, Kbd::new(*k)).horizontal().into_any_element()));
            }
            self.settings_group(Some(*title), fields)
        });
        div().children(groups)
    }
}
