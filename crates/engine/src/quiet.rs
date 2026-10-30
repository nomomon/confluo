//! Stale PRs and local snoozes, applied when listing from the cache.

use crate::{Engine, ListQuery};
use anyhow::Result;
use chrono::Utc;
use gv_core::{is_stale, ChangeRequest, ChangeRequestId, Snooze, SnoozeKind};
use std::collections::BTreeMap;

const SNOOZES_KEY: &str = "snoozes";

/// A list split into what shows in sections and what is quieted.
#[derive(Debug, Clone, Default)]
pub struct Listing {
    /// Active PRs for the sections.
    pub items: Vec<ChangeRequest>,
    /// Inactive longer than the stale threshold, newest first (empty with `include_stale`).
    pub stale: Vec<ChangeRequest>,
    /// Bot PRs left out by `q.bots`.
    pub hidden_bots: usize,
    /// PRs left out by a snooze or hide.
    pub snoozed: usize,
}

/// Split `items` into shown, stale and snoozed.
pub(crate) fn split(items: Vec<ChangeRequest>, q: &ListQuery, snoozes: &BTreeMap<String, Snooze>) -> (Vec<ChangeRequest>, Vec<ChangeRequest>, usize) {
    let now = Utc::now();
    let (mut shown, mut stale, mut snoozed) = (vec![], vec![], 0);
    for c in items {
        if snoozes.get(&c.id.key()).is_some_and(|s| s.hides(&c, now)) {
            snoozed += 1;
        } else if !q.include_stale && !q.starred_only && is_stale(&c, now, q.stale_days) {
            stale.push(c);
        } else {
            shown.push(c);
        }
    }
    (shown, stale, snoozed)
}

impl Engine {
    /// Local snoozes and hides by PR key.
    pub fn snoozes(&self) -> BTreeMap<String, Snooze> {
        self.store().get_meta(SNOOZES_KEY).ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    /// Snooze or hide a PR (`None` removes it, for undo). Returns the previous entry.
    pub fn set_snooze(&self, cr: &ChangeRequest, kind: Option<SnoozeKind>) -> Result<Option<Snooze>> {
        let mut all = self.snoozes();
        let key = cr.id.key();
        let prev = match kind {
            Some(k) => all.insert(key, Snooze::new(k, cr, Utc::now())),
            None => all.remove(&key),
        };
        self.store().set_meta(SNOOZES_KEY, &serde_json::to_string(&all)?)?;
        Ok(prev)
    }

    /// Restore a snooze entry exactly (undo).
    pub fn restore_snooze(&self, id: &ChangeRequestId, s: Option<Snooze>) -> Result<()> {
        let mut all = self.snoozes();
        match s {
            Some(s) => all.insert(id.key(), s),
            None => all.remove(&id.key()),
        };
        Ok(self.store().set_meta(SNOOZES_KEY, &serde_json::to_string(&all)?)?)
    }

    /// The list split into sections, stale and snoozed (see [`Listing`]).
    pub fn listing(&self, q: &ListQuery) -> Result<Listing> {
        let (items, hidden_bots) = self.list_counted(q)?;
        let (items, stale, snoozed) = split(items, q, &self.snoozes());
        Ok(Listing { items, stale, hidden_bots, snoozed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_core::*;

    fn cr(n: u64, age_days: i64) -> ChangeRequest {
        let t = Utc::now() - chrono::Duration::days(age_days);
        ChangeRequest {
            id: ChangeRequestId { repo: RepoRef::new(ProviderKind::GitHub, "github.com", "a/b"), number: n },
            title: format!("pr {n}"),
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
            needs_you: n == 2,
        }
    }

    #[test]
    fn splits_stale_and_snoozed() {
        let q = ListQuery { stale_days: Some(60), ..Default::default() };
        let mut sn = BTreeMap::new();
        let c4 = cr(4, 1);
        sn.insert(c4.id.key(), Snooze::new(SnoozeKind::Hidden, &c4, Utc::now()));
        let (shown, stale, snoozed) = split(vec![cr(1, 1), cr(2, 90), cr(3, 200), c4], &q, &sn);
        assert_eq!(shown.iter().map(|c| c.id.number).collect::<Vec<_>>(), vec![1]);
        assert_eq!(stale.len(), 2);
        assert_eq!(snoozed, 1);
        let q = ListQuery { stale_days: Some(60), include_stale: true, ..Default::default() };
        let (shown, stale, _) = split(vec![cr(1, 1), cr(2, 90)], &q, &BTreeMap::new());
        assert_eq!((shown.len(), stale.len()), (2, 0));
    }

    #[test]
    fn snooze_roundtrip_in_store() {
        let dir = std::env::temp_dir().join(format!("gv-engine-snooze-{}", std::process::id()));
        let e = Engine::open(&dir.join("t.db")).unwrap();
        let c = cr(1, 1);
        e.store().upsert_change_requests(&[c.clone(), cr(2, 1)]).unwrap();
        e.set_snooze(&c, Some(SnoozeKind::UntilUpdated)).unwrap();
        let l = e.listing(&ListQuery::default()).unwrap();
        assert_eq!((l.items.len(), l.snoozed), (1, 1));
        let prev = e.set_snooze(&c, None).unwrap();
        assert!(prev.is_some());
        assert_eq!(e.listing(&ListQuery::default()).unwrap().items.len(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
