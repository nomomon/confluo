//! Engine-to-UI mappings reused by several features.

pub mod ci;
mod format;
pub mod layout;
mod status;

pub use format::relative_time;
pub use status::{checks_text, row_hint, number_label, provider_icon, state_icon, state_icon_name};
