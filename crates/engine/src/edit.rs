//! Editing PRs (title, body, people, draft, state, merge) and check actions,
//! keeping the cached detail and list row in step.

use crate::Engine;
use anyhow::{bail, Result};
use gv_core::*;

fn noun(id: &ChangeRequestId) -> &'static str {
    if id.repo.kind == ProviderKind::GitLab {
        "MR"
    } else {
        "PR"
    }
}

impl Engine {
    /// Cached detail, fetched once when there is none.
    async fn detail_for_edit(&self, id: &ChangeRequestId) -> Result<ChangeRequestDetail> {
        match self.store().get_detail(id)? {
            Some(d) => Ok(d),
            None => self.detail(id, true).await,
        }
    }

    /// Save `d` to the cache (detail and list row) and mark it for a refetch.
    fn store_edit(&self, d: &ChangeRequestDetail) {
        let _ = self.store().put_detail(d);
        if let Ok(Some(mut cr)) = self.store().get_change_request(&d.summary.id) {
            cr.title = d.summary.title.clone();
            cr.state = d.summary.state;
            cr.reviewers = d.summary.reviewers.clone();
            let _ = self.store().upsert_change_requests(&[cr]);
        }
        self.forget_detail(&d.summary.id);
    }

    fn require(&self, ok: bool, what: &str, id: &ChangeRequestId) -> Result<()> {
        if !ok {
            bail!("{what} is not available on this {}", noun(id));
        }
        Ok(())
    }

    pub async fn update_change_request(
        &self,
        id: &ChangeRequestId,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<()> {
        let mut d = self.detail_for_edit(id).await?;
        let title = title.map(str::trim).filter(|t| !t.is_empty());
        self.require(
            title.is_none() || d.edit.caps.title,
            "Editing the title",
            id,
        )?;
        self.require(
            body.is_none() || d.edit.caps.body,
            "Editing the description",
            id,
        )?;
        self.provider(id.repo.kind, &id.repo.host)
            .await?
            .update_change_request(id, title, body)
            .await?;
        if let Some(t) = title {
            d.summary.title = t.to_string();
        }
        if let Some(b) = body {
            d.description = b.to_string();
        }
        self.store_edit(&d);
        Ok(())
    }

    /// Current reviewers or assignees of the cached detail.
    pub fn people(d: &ChangeRequestDetail, role: PeopleRole) -> Vec<User> {
        match role {
            PeopleRole::Reviewers => d.summary.reviewers.iter().map(|r| r.user.clone()).collect(),
            PeopleRole::Assignees => d.edit.assignees.clone(),
        }
    }

    /// Replace the reviewers or assignees with `desired`.
    pub async fn set_people(
        &self,
        id: &ChangeRequestId,
        role: PeopleRole,
        desired: Vec<User>,
    ) -> Result<()> {
        let mut d = self.detail_for_edit(id).await?;
        let allowed = match role {
            PeopleRole::Reviewers => d.edit.caps.reviewers,
            PeopleRole::Assignees => d.edit.caps.assignees,
        };
        self.require(
            allowed,
            &format!("Changing {}", role.label().to_lowercase()),
            id,
        )?;
        let current = Self::people(&d, role);
        self.provider(id.repo.kind, &id.repo.host)
            .await?
            .set_people(id, role, &current, &desired)
            .await?;
        match role {
            PeopleRole::Reviewers => {
                let mut next: Vec<Review> = d
                    .summary
                    .reviewers
                    .iter()
                    .filter(|r| {
                        desired
                            .iter()
                            .any(|u| u.username.eq_ignore_ascii_case(&r.user.username))
                    })
                    .cloned()
                    .collect();
                for u in &desired {
                    if !next
                        .iter()
                        .any(|r| r.user.username.eq_ignore_ascii_case(&u.username))
                    {
                        next.push(Review {
                            user: u.clone(),
                            state: ReviewState::Pending,
                        });
                    }
                }
                d.summary.reviewers = next;
            }
            PeopleRole::Assignees => d.edit.assignees = desired,
        }
        self.store_edit(&d);
        Ok(())
    }

    /// `+user -user` edits resolved against the current list and a user search (for ids).
    pub async fn people_from_args(
        &self,
        id: &ChangeRequestId,
        role: PeopleRole,
        args: &[String],
    ) -> Result<Vec<User>> {
        let d = self.detail_for_edit(id).await?;
        let current = Self::people(&d, role);
        let names = apply_people_args(
            &current
                .iter()
                .map(|u| u.username.clone())
                .collect::<Vec<_>>(),
            args,
        );
        let mut out = vec![];
        for n in names {
            if let Some(u) = current.iter().find(|u| u.username.eq_ignore_ascii_case(&n)) {
                out.push(u.clone());
                continue;
            }
            let found = self.search_users(id, &n).await?;
            match found
                .into_iter()
                .find(|u| u.username.eq_ignore_ascii_case(&n))
            {
                Some(u) => out.push(u),
                None => bail!("@{n} can't be added to this {}", noun(id)),
            }
        }
        Ok(out)
    }

    pub async fn search_users(&self, id: &ChangeRequestId, query: &str) -> Result<Vec<User>> {
        Ok(self
            .provider(id.repo.kind, &id.repo.host)
            .await?
            .search_users(id, query.trim())
            .await?)
    }

    pub async fn set_draft(&self, id: &ChangeRequestId, draft: bool) -> Result<()> {
        let mut d = self.detail_for_edit(id).await?;
        self.require(d.edit.caps.draft, "Changing draft state", id)?;
        let p = self.provider(id.repo.kind, &id.repo.host).await?;
        p.set_draft(id, draft, &d.summary.title, d.edit.node_id.as_deref())
            .await?;
        d.summary.state = if draft {
            ChangeRequestState::Draft
        } else {
            ChangeRequestState::Open
        };
        if id.repo.kind == ProviderKind::GitLab {
            d.summary.title = draft_title(&d.summary.title, draft);
        }
        self.store_edit(&d);
        Ok(())
    }

    pub async fn set_open(&self, id: &ChangeRequestId, open: bool) -> Result<()> {
        let mut d = self.detail_for_edit(id).await?;
        self.require(
            if open {
                d.edit.caps.reopen
            } else {
                d.edit.caps.close
            },
            if open { "Reopening" } else { "Closing" },
            id,
        )?;
        self.provider(id.repo.kind, &id.repo.host)
            .await?
            .set_open(id, open)
            .await?;
        d.summary.state = if open {
            ChangeRequestState::Open
        } else {
            ChangeRequestState::Closed
        };
        self.store_edit(&d);
        Ok(())
    }

    pub async fn merge(&self, id: &ChangeRequestId, opts: &MergeOptions) -> Result<()> {
        let mut d = self.detail_for_edit(id).await?;
        if !d.edit.caps.merge {
            bail!(
                "{}",
                d.edit
                    .caps
                    .merge_blocked
                    .clone()
                    .unwrap_or_else(|| format!("You can't merge this {}", noun(id)))
            );
        }
        self.provider(id.repo.kind, &id.repo.host)
            .await?
            .merge(id, opts)
            .await?;
        if !opts.when_pipeline_succeeds {
            d.summary.state = ChangeRequestState::Merged;
        }
        self.store_edit(&d);
        Ok(())
    }

    pub async fn check_log(&self, id: &ChangeRequestId, run: &CheckRun) -> Result<Vec<String>> {
        Ok(self
            .provider(id.repo.kind, &id.repo.host)
            .await?
            .check_log(id, run)
            .await?)
    }

    /// Re-run the failed checks of the cached detail.
    pub async fn rerun_failed(&self, id: &ChangeRequestId) -> Result<()> {
        let d = self.detail_for_edit(id).await?;
        let Some(target) = d.checks.rerun_target.clone().filter(|_| d.checks.can_rerun) else {
            bail!("Nothing to re-run on this {}", noun(id))
        };
        self.provider(id.repo.kind, &id.repo.host)
            .await?
            .rerun_failed(id, &target)
            .await?;
        self.forget_detail(id);
        Ok(())
    }
}
