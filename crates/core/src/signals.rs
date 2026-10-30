//! Per-viewer facts beyond reviewer states that decide whether a PR needs me.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signals {
    /// Commits pushed after my last review. None: I have not reviewed, or it is unknown.
    /// GitHub: my last review's commit vs the head; GitLab: "added N commits" notes after my
    /// last review; Azure: commits of iterations pushed after my vote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_my_review: Option<u32>,
    /// The source branch conflicts with the target.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub conflicts: bool,
    /// Unresolved threads waiting on me (known once the detail was loaded).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub threads_need_me: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}
