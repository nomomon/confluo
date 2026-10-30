//! Submitting reviews and keeping the cached detail in step.

use crate::Engine;
use anyhow::{bail, Result};
use gv_core::{state_after, ChangeRequestId, ReviewAction};

impl Engine {
    /// Submit a review. Checks the cached capabilities first, then marks the cached detail
    /// with the new state and forgets its freshness so the next `detail` call refetches.
    pub async fn submit_review(&self, id: &ChangeRequestId, action: ReviewAction, body: Option<&str>) -> Result<()> {
        let cached = self.store().get_detail(id)?;
        if let Some(d) = &cached {
            if !d.review.caps.allows(action) {
                match &d.review.caps.approve_blocked {
                    Some(why) if action == ReviewAction::Approve => bail!("{why}"),
                    _ => bail!("{} is not available on this {}", action.label(), if id.repo.kind == gv_core::ProviderKind::GitLab { "MR" } else { "PR" }),
                }
            }
        }
        let body = body.map(str::trim).filter(|b| !b.is_empty());
        self.provider(id.repo.kind, &id.repo.host).await?.submit_review(id, action, body).await?;
        if let Some(mut d) = cached {
            d.review.my_state = state_after(action, d.review.my_state);
            let _ = self.store().put_detail(&d);
        }
        self.forget_detail(id);
        Ok(())
    }
}
