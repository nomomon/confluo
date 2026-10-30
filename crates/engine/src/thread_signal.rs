//! "Threads waiting on me" for list rows: counted when a detail loads, kept until the PR changes.

use gv_core::{
    compute_needs_you, threads_attention, Attention, ChangeRequest, ChangeRequestDetail,
};

use crate::Engine;

/// Unresolved threads waiting on `me` in a loaded detail.
pub(crate) fn threads_need_me(d: &ChangeRequestDetail, me: Option<&str>) -> u32 {
    threads_attention(d, me)
        .into_iter()
        .filter(|a| *a == Attention::NeedsMe)
        .count() as u32
}

/// Keep `cached`'s thread count on a freshly listed `cr` while the PR has not changed since.
pub(crate) fn carry(cr: &mut ChangeRequest, cached: &ChangeRequest, me: Option<&str>) {
    if cached.updated_at != cr.updated_at || cached.signals.threads_need_me == 0 {
        return;
    }
    cr.signals.threads_need_me = cached.signals.threads_need_me;
    if let Some(me) = me {
        cr.needs_you = compute_needs_you(cr, me);
    }
}

impl Engine {
    /// Before a list upsert: carry cached thread counts (lists never include threads).
    pub(crate) fn carry_threads(&self, items: &mut [ChangeRequest]) {
        for cr in items {
            if let Ok(Some(c)) = self.store().get_change_request(&cr.id) {
                let me = self.me(cr.id.repo.kind, &cr.id.repo.host);
                carry(cr, &c, me.as_deref());
            }
        }
    }

    /// After a detail fetch: count threads waiting on me and update the list row.
    pub(crate) fn note_threads(&self, d: &mut ChangeRequestDetail) {
        let me = self.me(d.summary.id.repo.kind, &d.summary.id.repo.host);
        d.summary.signals.threads_need_me = threads_need_me(d, me.as_deref());
        if let Some(me) = &me {
            d.summary.needs_you = compute_needs_you(&d.summary, me);
        }
        if let Ok(Some(mut cr)) = self.store().get_change_request(&d.summary.id) {
            cr.signals.threads_need_me = d.summary.signals.threads_need_me;
            if d.summary.signals.since_my_review.is_some() {
                cr.signals.since_my_review = d.summary.signals.since_my_review;
            }
            cr.reviewers = d.summary.reviewers.clone();
            if let Some(me) = &me {
                cr.needs_you = compute_needs_you(&cr, me);
            }
            let _ = self.store().upsert_change_requests(&[cr]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn carries_only_while_unchanged() {
        let mut cached = crate::tests::sample();
        cached.signals.threads_need_me = 2;
        let mut fresh = cached.clone();
        fresh.signals.threads_need_me = 0;
        carry(&mut fresh, &cached, Some("me"));
        assert_eq!(fresh.signals.threads_need_me, 2);
        assert!(fresh.needs_you);
        let mut moved = cached.clone();
        moved.signals.threads_need_me = 0;
        moved.updated_at = cached.updated_at + Duration::minutes(1);
        carry(&mut moved, &cached, Some("me"));
        assert_eq!(moved.signals.threads_need_me, 0);
    }
}
