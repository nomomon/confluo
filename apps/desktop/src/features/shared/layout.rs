//! Fixed layout dimensions of the window regions.

use gpui::{px, Pixels};

pub const SIDEBAR_W: Pixels = px(220.);
pub const DETAIL_W: Pixels = px(540.);
/// Top padding of the sidebar, clearing the traffic lights.
pub const SIDEBAR_TOP: Pixels = px(44.);
/// Top padding of the list and detail columns, under the transparent titlebar.
pub const TITLEBAR_INSET: Pixels = px(36.);
pub const TIME_COL_W: Pixels = px(56.);
pub const TINT_INPUT_W: Pixels = px(110.);
pub const ALIAS_LIST_MAX_H: Pixels = px(300.);
