//! App state transitions and the async bridge to the engine.
//!
//! Each file adds an `impl GitViewer` block for one concern; rendering lives in `features/`.

pub mod bridge;
mod comments;
mod detail;
mod list;
mod projects;

pub use detail::{DetailState, DetailTab};
pub use projects::SourceKind;

/// Format an error with its full context chain.
pub fn err_str(e: anyhow::Error) -> String {
    format!("{e:#}")
}
