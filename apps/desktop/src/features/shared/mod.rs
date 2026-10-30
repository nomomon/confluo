//! Engine-to-UI mappings reused by several features.

pub mod ci;
mod format;
pub mod layout;
mod status;

pub use format::relative_time;
pub use status::{
    checks_text, number_label, provider_icon, provider_mark, row_hint, state_icon, state_icon_name,
};
