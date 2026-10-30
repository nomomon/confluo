//! Shared look of every selectable list row: a rounded highlight that never touches its
//! container's edges. Sidebar nav, menus, the palette, selects, the file tree and PR rows all
//! use it, so hover and selection read the same everywhere.
//!
//! `div().id("row").list_item(selected)`; `.list_item_radius(theme::LIST_ITEM_RADIUS)` for a
//! different corner. Rows inside a card sit in a container padded by `theme::LIST_INSET`.

use gpui::{prelude::*, Div, Pixels, Stateful};

use crate::ui::theme;

pub trait ListItem: Sized {
    /// Rounded `RADIUS_MD` row: selected fill, hover and press states.
    fn list_item(self, selected: bool) -> Self {
        self.list_item_radius(selected, theme::LIST_ITEM_RADIUS)
    }
    fn list_item_radius(self, selected: bool, radius: Pixels) -> Self;
}

impl ListItem for Stateful<Div> {
    fn list_item_radius(self, selected: bool, radius: Pixels) -> Self {
        self.rounded(radius)
            .cursor_pointer()
            .when(selected, |d| d.bg(theme::surface_active()))
            .hover(|s| {
                s.bg(if selected {
                    theme::surface_active()
                } else {
                    theme::surface_hover()
                })
            })
            .active(|s| s.bg(theme::surface_active()))
    }
}
