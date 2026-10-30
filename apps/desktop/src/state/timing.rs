//! Non-animation timings: refresh cadence and feedback resets.

use std::time::Duration;

/// How often the clock ticks to re-render relative times and check the refresh interval setting.
pub const TICK: Duration = Duration::from_secs(30);
/// Minimum gap between refreshes triggered by the window becoming active.
pub const ACTIVATE_MIN_GAP: Duration = Duration::from_secs(2 * 60);
/// Repeated manual refreshes of the same view within this gap are ignored.
pub const MANUAL_DEBOUNCE: Duration = Duration::from_secs(5);
/// Selecting a view whose data is newer than this shows the cache without refreshing.
pub const SELECT_FRESH: Duration = Duration::from_secs(2 * 60);
/// Loading indicators appear only if work takes longer than this.
pub const LOADING_DELAY: Duration = Duration::from_millis(150);
/// "Copied" feedback and toasts reset after this.
pub const COPIED_RESET: Duration = Duration::from_millis(1500);
pub const TOAST: Duration = Duration::from_millis(2500);
/// Error toasts stay longer so the reason can be read.
pub const TOAST_ERROR: Duration = Duration::from_millis(6000);
/// Holding Cmd alone this long shows the shortcut hints.
pub const HINT_HOLD: Duration = Duration::from_millis(250);
/// Typing pause before a remote user search.
pub const SEARCH_DEBOUNCE: Duration = Duration::from_millis(250);
/// How long Accounts waits for a host to answer "who am I" before keeping the cached user.
pub const HOST_CHECK_TIMEOUT: Duration = Duration::from_secs(8);
/// Clicking the previewed row again hides the preview only after this, so a double click opens the
/// review without hiding the preview first.
pub const DOUBLE_CLICK: Duration = Duration::from_millis(350);
/// A press outside a floating layer closes it; a click on its own trigger within this time (the
/// same gesture) keeps it closed instead of reopening it.
pub const LAYER_REOPEN_GUARD: Duration = Duration::from_millis(400);
/// How often a running app checks GitHub for a newer release.
pub const UPDATE_EVERY: Duration = Duration::from_secs(6 * 3600);
/// How often download progress is redrawn.
pub const UPDATE_PROGRESS: Duration = Duration::from_millis(150);
