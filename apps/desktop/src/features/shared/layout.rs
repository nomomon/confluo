//! Fixed layout dimensions of the window regions.

use gpui::{px, Pixels};

pub const SIDEBAR_W: Pixels = px(240.);
/// How far the sidebar slides in when shown again (Cmd+B).
pub const SIDEBAR_SLIDE: Pixels = px(24.);
/// Sidebar collapsed to an icon rail, used while the detail pane is open in a narrow window.
pub const SIDEBAR_RAIL_W: Pixels = px(60.);
/// Below this window width an open detail pane collapses the sidebar to the rail.
pub const COMPACT_BELOW: Pixels = px(1440.);
/// The list keeps at least this width; the detail pane gives way first.
pub const LIST_MIN_W: Pixels = px(420.);
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
/// Settings dialog size, its left navigation and the background image thumbnail.
pub const SETTINGS_W: Pixels = px(780.);
pub const SETTINGS_H: Pixels = px(560.);
pub const SETTINGS_NAV_W: Pixels = px(188.);
pub const SETTINGS_THUMB_W: Pixels = px(72.);
pub const SETTINGS_THUMB_H: Pixels = px(44.);
/// Repo column of the alias table.
pub const ALIAS_REPO_W: Pixels = px(260.);
/// Code tab: one diff line, the file card header, the `@@` band and line number gutters.
pub const DIFF_LINE_H: Pixels = px(20.);
pub const DIFF_HEADER_H: Pixels = px(40.);
pub const DIFF_HUNK_H: Pixels = px(28.);
pub const DIFF_GUTTER_W: Pixels = px(40.);
/// Width of the Code tab's file tree.
pub const FILE_TREE_W: Pixels = px(200.);
/// Approximate mono glyph width as a fraction of the font size, for horizontal scroll limits.
pub const MONO_CHAR_RATIO: f32 = 0.62;
/// Narrowest width of the add field after the chips in the Filters chip editors.
pub const CHIP_INPUT_MIN_W: Pixels = px(240.);
