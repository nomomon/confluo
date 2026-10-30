//! Shortcuts: read-only list of key bindings, straight from the keymap (same rows as the `?` sheet).

use gpui::{div, prelude::*};

use crate::app::GitViewer;
use crate::features::keymap::{group_rows, Group};
use crate::ui::atoms::Kbd;
use crate::ui::molecules::Field;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_shortcuts(&self) -> impl IntoElement {
        let groups = Group::ALL.into_iter().map(|g| {
            let fields = group_rows(g)
                .into_iter()
                .map(|(label, keys)| {
                    let row = div()
                        .flex()
                        .gap(theme::SPACE_1P5)
                        .children(keys.into_iter().map(Kbd::new));
                    Field::new(label, row).horizontal().into_any_element()
                })
                .collect();
            self.settings_group(Some(g.title()), fields)
        });
        div().children(groups)
    }
}
