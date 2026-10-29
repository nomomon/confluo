//! Domain types and the provider trait. No IO here.

mod diff;
mod inline;
mod model;
mod needs_you;
mod provider;

pub use diff::*;
pub use inline::{index_inline_threads, InlineIndex, LineKey};
pub use model::*;
pub use needs_you::compute_needs_you;
pub use provider::{Provider, ProviderError, ProviderResult};
