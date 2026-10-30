//! General preferences: refresh cadence, where PRs open and what shows at launch.

use serde::{Deserialize, Serialize};

/// Auto refresh choices in minutes; 0 means manual only.
pub const REFRESH_MINUTES: [u32; 7] = [1, 2, 5, 10, 15, 30, 0];

pub fn refresh_label(m: u32) -> String {
    match m {
        0 => "Manually".into(),
        1 => "Every minute".into(),
        m => format!("Every {m} minutes"),
    }
}

/// Where a click on a list row opens the PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OpenIn {
    #[default]
    App,
    Browser,
}

/// What the window shows at launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Launch {
    #[default]
    Inbox,
    /// The project that was selected when the app quit.
    LastView,
}

impl Launch {
    pub const ALL: [Launch; 2] = [Launch::Inbox, Launch::LastView];
    pub fn label(self) -> &'static str {
        match self {
            Launch::Inbox => "Open Inbox",
            Launch::LastView => "Restore last view",
        }
    }
}
