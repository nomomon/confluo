//! Domain types and the provider trait. No IO here.

mod attention;
mod bots;
mod checks;
mod diff;
mod edit;
mod inline;
mod model;
mod needs_you;
mod provider;
mod reactions;
mod review;
mod row_ref;
mod stale;
mod suggestion;

pub use attention::{thread_attention, threads_attention, Attention};
pub use bots::{glob, BotFilter, DEFAULT_BOT_AUTHORS, DEFAULT_BOT_BRANCHES};
pub use checks::*;
pub use diff::*;
pub use edit::*;
pub use inline::{index_inline_threads, InlineIndex, LineKey};
pub use model::*;
pub use needs_you::compute_needs_you;
pub use provider::{Provider, ProviderError, ProviderResult};
pub use reactions::*;
pub use review::*;
pub use row_ref::{row_ref, RowRef};
pub use stale::*;
pub use suggestion::*;
