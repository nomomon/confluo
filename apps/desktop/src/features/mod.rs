//! Features: app-aware organisms and views built from `ui/` components.
//!
//! Each module adds `render_*` methods to `GitViewer` for one area of the window.

mod code_keys;
mod commands;
mod hints;
pub(crate) mod keyboard;
pub(crate) mod keymap;
pub(crate) mod menus;
mod overlay;
mod palette;
mod pr_detail;
mod pr_list;
mod projects;
mod repo_menu;
mod review_dialog;
mod settings;
mod shared;
mod shortcuts_sheet;
pub(crate) use shared::layout;
mod sidebar;
mod swipe_nav;
mod view_commands;
