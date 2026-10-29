//! App state transitions and the async bridge to the engine.
//!
//! Each file adds an `impl GitViewer` block for one concern; rendering lives in `features/`.

mod avatars;
pub mod bridge;
mod comments;
mod connection;
mod detail;
mod feedback;
mod inbox;
mod list;
mod overlay;
mod projects;
mod selection;
mod sources;
pub mod timing;

pub use connection::{auth_command, HostStatus};
pub use detail::{DetailState, DetailTab};
pub use feedback::Copied;
pub use inbox::Section;
pub use selection::reveal;
pub use sources::SourceKind;
pub use overlay::{Menu, Overlay};

/// Format an error with its full context chain.
pub fn err_str(e: anyhow::Error) -> String {
    format!("{e:#}")
}
