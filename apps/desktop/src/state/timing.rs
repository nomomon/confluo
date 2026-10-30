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
