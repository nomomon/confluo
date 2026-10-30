//! Features: app-aware organisms and views built from `ui/` components.
//!
//! Each module adds `render_*` methods to `GitViewer` for one area of the window.

mod code_keys;
pub(crate) mod keyboard;
pub(crate) mod keymap;
mod commands;
mod palette;
pub(crate) mod menus;
mod shortcuts_sheet;
mod review_dialog;
mod overlay;
mod pr_detail;
mod pr_list;
mod projects;
mod repo_menu;
mod settings;
mod shared;
pub(crate) use shared::layout;
mod sidebar;
