//! App state transitions and the async bridge to the engine.
//!
//! Each file adds an `impl GitViewer` block for one concern; rendering lives in `features/`.

mod avatars;
pub mod bridge;
mod code;
mod code_load;
pub mod code_rows;
mod comments;
mod connection;
mod detail;
mod dev_hooks;
mod feedback;
mod file_view;
mod inbox;
mod inline;
mod line_actions;
mod line_comments;
mod line_cursor;
pub mod line_select;
mod permalink;
mod list;
mod overlay;
mod nav;
mod review;
mod projects;
mod quiet;
mod selection;
mod sources;
pub mod timing;

pub use connection::{auth_command, host_avatar_url, HostStatus};
pub use code::CodeState;
pub use detail::{DetailState, DetailTab};
pub use file_view::FileView;
pub use feedback::Copied;
pub use inbox::Section;
pub use inline::LineDraft;
pub use selection::reveal;
pub use sources::SourceKind;
pub use quiet::QuietState;
pub use nav::NavState;
pub use overlay::{BotList, Menu, Overlay, SettingsPage};

/// Format an error with its full context chain.
pub fn err_str(e: anyhow::Error) -> String {
    format!("{e:#}")
}
