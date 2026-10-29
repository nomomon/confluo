//! Domain types and the provider trait. No IO here.

mod diff;
mod model;
mod needs_you;
mod provider;

pub use diff::*;
pub use model::*;
pub use needs_you::compute_needs_you;
pub use provider::{Provider, ProviderError, ProviderResult};
