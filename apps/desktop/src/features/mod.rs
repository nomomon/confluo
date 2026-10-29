//! Features: app-aware organisms and views built from `ui/` components.
//!
//! Each module adds `render_*` methods to `GitViewer` for one area of the window.

mod aliases;
mod overlay;
mod pr_detail;
mod pr_list;
mod projects;
mod settings;
mod shared;
pub(crate) use shared::layout;
mod sidebar;
