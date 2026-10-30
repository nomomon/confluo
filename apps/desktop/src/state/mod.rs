//! App state transitions and the async bridge to the engine.
//!
//! Each file adds an `impl GitViewer` block for one concern; rendering lives in `features/`.

mod attention;
mod avatars;
mod awards;
pub mod bridge;
mod check_actions;
pub mod cmd_ctx;
mod code;
mod code_load;
pub mod code_rows;
mod comments;
mod connection;
mod detail;
mod dev_hooks;
mod feedback;
mod file_tree;
mod file_view;
pub mod find;
mod find_nav;
pub mod hint_hold;
mod inbox;
mod inline;
mod line_actions;
mod line_comments;
mod line_cursor;
pub mod line_select;
mod list;
mod nav;
mod overlay;
pub mod overlay_close;
pub mod palette_rank;
mod people;
mod permalink;
pub mod pr_edit;
pub mod pr_nav;
mod projects;
mod quiet;
mod reactions;
mod repo_menu;
mod review;
mod review_mode;
mod selection;
mod sources;
mod split;
mod suggestions;
mod swipe_nav;
pub mod timing;
mod update;
mod view_prefs;
pub(crate) use update::UpdateState;

pub use attention::{attention, ThreadFilter};
pub use code::CodeState;
pub use connection::{auth_command, host_avatar_url, sign_in_hint, HostStatus};
pub use detail::{DetailState, DetailTab};
pub use feedback::Copied;
pub use file_tree::{tree_entries, TreeEntry, TreeQuery};
pub use file_view::FileView;
pub use find::FindState;
pub use inbox::Section;
pub use inline::LineDraft;
pub use nav::NavState;
pub use overlay::{BotList, Layer, Overlay, SettingsPage};
pub use pr_edit::{EditUi, LogState};
pub use quiet::QuietState;
pub use repo_menu::RepoMenu;
pub use review_mode::{Pane, ReviewUi, RightTab};
pub use selection::reveal;
pub use sources::SourceKind;
pub use swipe_nav::SwipeUi;

/// Format an error with its full context chain.
pub fn err_str(e: anyhow::Error) -> String {
    format!("{e:#}")
}
