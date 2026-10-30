//! Quieting old PRs: staleness by inactivity, and per-PR snoozes or hides kept locally.

use crate::ChangeRequest;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Days of inactivity after which a PR counts as stale, by default.
pub const DEFAULT_STALE_DAYS: u32 = 60;
/// Choices offered in Settings; `None` means never.
pub const STALE_CHOICES: [Option<u32>; 5] = [Some(30), Some(60), Some(90), Some(180), None];

/// A PR not updated for more than `days` days is stale. `None` disables staleness.
/// A pending review request does not exempt it: once past the threshold it is stale too.
pub fn is_stale(cr: &ChangeRequest, now: DateTime<Utc>, days: Option<u32>) -> bool {
    days.is_some_and(|d| now - cr.updated_at > Duration::days(d as i64))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnoozeKind {
    /// Hidden for a week, or until updated, whichever comes first.
    Week,
    /// Hidden until the PR is updated.
    UntilUpdated,
    /// Hidden for good (until unhidden).
    Hidden,
}

/// A local snooze on one PR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snooze {
    pub kind: SnoozeKind,
    /// The PR's `updated_at` when snoozed; a later update brings it back.
    pub at_updated: DateTime<Utc>,
    /// End of a timed snooze.
    pub until: Option<DateTime<Utc>>,
}

impl Snooze {
    pub fn new(kind: SnoozeKind, cr: &ChangeRequest, now: DateTime<Utc>) -> Self {
        let until = (kind == SnoozeKind::Week).then(|| now + Duration::weeks(1));
        Self {
            kind,
            at_updated: cr.updated_at,
            until,
        }
    }

    /// Whether this snooze still hides `cr` at `now`.
    pub fn hides(&self, cr: &ChangeRequest, now: DateTime<Utc>) -> bool {
        let updated = cr.updated_at > self.at_updated;
        match self.kind {
            SnoozeKind::Hidden => true,
            SnoozeKind::UntilUpdated => !updated,
            SnoozeKind::Week => !updated && self.until.is_some_and(|u| now < u),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn cr(age_days: i64) -> ChangeRequest {
        let t = Utc::now() - Duration::days(age_days);
        ChangeRequest {
            id: ChangeRequestId {
                repo: RepoRef::new(ProviderKind::GitHub, "github.com", "a/b"),
                number: 1,
            },
            title: "t".into(),
            author: User::named("a"),
            state: ChangeRequestState::Open,
            source_branch: "f".into(),
            target_branch: "main".into(),
            reviewers: vec![],
            checks: CheckSummary::default(),
            labels: vec![],
            comments_count: 0,
            additions: None,
            deletions: None,
            changed_files: None,
            updated_at: t,
            created_at: t,
            web_url: String::new(),
            needs_you: true,
        }
    }

    #[test]
    fn stale_by_threshold_even_if_needs_you() {
        let now = Utc::now();
        assert!(is_stale(&cr(61), now, Some(60)));
        assert!(!is_stale(&cr(10), now, Some(60)));
        assert!(!is_stale(&cr(400), now, None));
    }

    #[test]
    fn snoozes_end_on_update_or_time() {
        let now = Utc::now();
        let mut c = cr(3);
        let week = Snooze::new(SnoozeKind::Week, &c, now);
        let upd = Snooze::new(SnoozeKind::UntilUpdated, &c, now);
        let hide = Snooze::new(SnoozeKind::Hidden, &c, now);
        assert!(week.hides(&c, now) && upd.hides(&c, now) && hide.hides(&c, now));
        assert!(!week.hides(&c, now + Duration::days(8)));
        assert!(upd.hides(&c, now + Duration::days(80)));
        c.updated_at = now;
        assert!(!week.hides(&c, now) && !upd.hides(&c, now));
        assert!(hide.hides(&c, now));
    }
}
