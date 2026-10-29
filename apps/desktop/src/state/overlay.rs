//! Which popover menu or modal dialog is open.

/// Which popover menu is open.
#[derive(Clone, Copy, PartialEq)]
pub enum Menu {
    DetailMore,
    Project(i64),
}

/// Which modal dialog is open.
#[derive(Clone, Copy, PartialEq)]
pub enum Overlay {
    None,
    Settings,
    AddProject,
    Alias,
    Rename(i64),
}
