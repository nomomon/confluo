//! Fixed layout dimensions of the window regions.

use gpui::{px, Pixels};

pub const SIDEBAR_W: Pixels = px(240.);
/// Max width of the centered list content column.
pub const CONTENT_MAX_W: Pixels = px(1360.);
pub const DETAIL_W: Pixels = px(540.);
pub const DETAIL_MIN_W: Pixels = px(380.);
pub const DETAIL_MAX_W: Pixels = px(900.);
/// Hit area of the detail pane's resize handle.
pub const RESIZE_HANDLE_W: Pixels = px(6.);
/// Top padding of the sidebar, clearing the traffic lights.
pub const SIDEBAR_TOP: Pixels = px(52.);
/// Top padding of the list and detail columns, under the transparent titlebar.
pub const TITLEBAR_INSET: Pixels = px(48.);
pub const TIME_COL_W: Pixels = px(72.);
pub const ALIAS_LIST_MAX_H: Pixels = px(300.);
