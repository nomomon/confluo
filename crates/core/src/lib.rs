//! Domain types and the provider trait. No IO here.

mod bots;
mod checks;
mod edit;
mod diff;
mod inline;
mod model;
mod needs_you;
mod provider;
mod review;
mod stale;

pub use bots::{glob, BotFilter, DEFAULT_BOT_AUTHORS, DEFAULT_BOT_BRANCHES};
pub use checks::*;
pub use diff::*;
pub use edit::*;
pub use inline::{index_inline_threads, InlineIndex, LineKey};
pub use model::*;
pub use needs_you::compute_needs_you;
pub use provider::{Provider, ProviderError, ProviderResult};
pub use review::*;
pub use stale::*;
