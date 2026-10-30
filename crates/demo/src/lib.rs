//! In-memory fixture providers for demo mode (`CONFLUO_DEMO=1`): invented orgs, people and PRs.
//! Nothing here touches the network; writes change the in-memory copy only.

mod avatars;
mod catalog;
mod detail;
mod diffs;
mod patch;
mod people;
mod threads;

use async_trait::async_trait;
use chrono::Utc;
use gv_core::*;
use std::collections::HashMap;
use std::sync::Mutex;

pub use avatars::avatar;
pub use catalog::{all as change_requests, GITHUB, GITLAB};
pub use people::ME;

/// Projects created in a fresh demo cache: (name, kind, host, group).
pub const PROJECTS: &[(&str, ProviderKind, &str, &str)] = &[
    ("Acme", ProviderKind::GitHub, GITHUB, "acme"),
    ("Orbit", ProviderKind::GitLab, GITLAB, "orbit"),
];

/// One fixture host.
pub struct DemoProvider {
    kind: ProviderKind,
    host: String,
    details: Mutex<HashMap<ChangeRequestId, ChangeRequestDetail>>,
}

impl DemoProvider {
    pub fn new(kind: ProviderKind, host: &str) -> Self {
        Self {
            kind,
            host: host.into(),
            details: Mutex::default(),
        }
    }

    fn with_detail<T>(
        &self,
        id: &ChangeRequestId,
        f: impl FnOnce(&mut ChangeRequestDetail) -> T,
    ) -> ProviderResult<T> {
        let mut map = self.details.lock().unwrap_or_else(|e| e.into_inner());
        if !map.contains_key(id) {
            let cr = catalog::find(id).ok_or_else(|| ProviderError::NotFound(id.to_string()))?;
            map.insert(id.clone(), detail::build(cr));
        }
        Ok(f(map.get_mut(id).expect("inserted above")))
    }

    fn comment(body: &str, position: Option<InlinePosition>) -> Comment {
        let id = format!(
            "local-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        Comment {
            id,
            author: people::user(ME),
            body: body.into(),
            created_at: Utc::now(),
            position,
            reactions: vec![],
            suggestions: vec![],
        }
    }
}

fn involves_me(cr: &ChangeRequest) -> bool {
    cr.author.username == ME || cr.reviewers.iter().any(|r| r.user.username == ME)
}

#[async_trait]
impl Provider for DemoProvider {
    fn kind(&self) -> ProviderKind {
        self.kind
    }
    fn host(&self) -> &str {
        &self.host
    }
    async fn whoami(&self) -> ProviderResult<User> {
        Ok(people::user(ME))
    }
    async fn list_change_requests(
        &self,
        scope: &Scope,
        filter: &Filter,
    ) -> ProviderResult<Vec<ChangeRequest>> {
        let all = catalog::on_host(self.kind, &self.host)
            .into_iter()
            .filter(|c| filter.state.matches(c.state));
        Ok(match scope {
            Scope::InvolvingMe { .. } => all.filter(involves_me).collect(),
            Scope::Group { owner, .. } => all
                .filter(|c| c.id.repo.path.starts_with(&format!("{owner}/")))
                .collect(),
            Scope::Repo(r) => all.filter(|c| &c.id.repo == r).collect(),
        })
    }
    async fn get_change_request(
        &self,
        id: &ChangeRequestId,
    ) -> ProviderResult<ChangeRequestDetail> {
        self.with_detail(id, |d| d.clone())
    }
    async fn list_repos(&self, owner: &str) -> ProviderResult<Vec<RepoRef>> {
        Ok(catalog::repos(self.kind, &self.host, owner))
    }
    async fn add_comment(&self, id: &ChangeRequestId, body: &str) -> ProviderResult<Comment> {
        let c = Self::comment(body, None);
        self.with_detail(id, |d| {
            d.threads
                .push(Thread::conversation(c.id.clone(), vec![c.clone()]))
        })?;
        Ok(c)
    }
    async fn reply_to_thread(
        &self,
        id: &ChangeRequestId,
        thread_id: &str,
        body: &str,
    ) -> ProviderResult<Comment> {
        let c = Self::comment(body, None);
        self.with_detail(id, |d| {
            d.threads
                .iter_mut()
                .filter(|t| t.id == thread_id)
                .for_each(|t| t.comments.push(c.clone()))
        })?;
        Ok(c)
    }
    async fn edit_comment(
        &self,
        id: &ChangeRequestId,
        comment_id: &str,
        body: &str,
    ) -> ProviderResult<Comment> {
        self.with_detail(id, |d| {
            let c = d
                .threads
                .iter_mut()
                .flat_map(|t| t.comments.iter_mut())
                .find(|c| c.id == comment_id);
            c.map(|c| {
                c.body = body.into();
                c.clone()
            })
        })?
        .ok_or_else(|| ProviderError::NotFound(comment_id.into()))
    }
    async fn add_line_comment(
        &self,
        id: &ChangeRequestId,
        position: &DiffPosition,
        body: &str,
    ) -> ProviderResult<Comment> {
        let c = Self::comment(
            body,
            Some(InlinePosition {
                path: position.path.clone(),
                line: Some(position.line),
            }),
        );
        let t = Thread {
            id: c.id.clone(),
            resolved: Some(false),
            comments: vec![c.clone()],
            position: Some(position.clone()),
            resolved_by: None,
            resolved_at: None,
            outdated: false,
        };
        self.with_detail(id, |d| d.threads.push(t))?;
        Ok(c)
    }
    async fn resolve_thread(
        &self,
        id: &ChangeRequestId,
        thread_id: &str,
        resolved: bool,
    ) -> ProviderResult<()> {
        self.with_detail(id, |d| {
            d.threads
                .iter_mut()
                .filter(|t| t.id == thread_id)
                .for_each(|t| t.resolved = Some(resolved))
        })
    }
    async fn submit_review(
        &self,
        _id: &ChangeRequestId,
        _action: ReviewAction,
        _body: Option<&str>,
    ) -> ProviderResult<()> {
        Ok(())
    }
    async fn search_users(&self, _id: &ChangeRequestId, query: &str) -> ProviderResult<Vec<User>> {
        Ok(people::search(query))
    }
    async fn react(
        &self,
        id: &ChangeRequestId,
        subject: &str,
        kind: ReactionKind,
        _add: bool,
    ) -> ProviderResult<()> {
        self.with_detail(id, |d| {
            if subject.is_empty() {
                toggle_reaction(&mut d.reactions, kind, ME);
            } else if let Some(c) = d
                .threads
                .iter_mut()
                .flat_map(|t| t.comments.iter_mut())
                .find(|c| c.id == subject)
            {
                toggle_reaction(&mut c.reactions, kind, ME);
            }
        })
    }
    async fn comment_reactions(
        &self,
        id: &ChangeRequestId,
        comment_id: &str,
    ) -> ProviderResult<Vec<ReactionGroup>> {
        self.with_detail(id, |d| {
            d.threads
                .iter()
                .flat_map(|t| &t.comments)
                .find(|c| c.id == comment_id)
                .map(|c| c.reactions.clone())
                .unwrap_or_default()
        })
    }
    async fn check_log(
        &self,
        _id: &ChangeRequestId,
        run: &CheckRun,
    ) -> ProviderResult<Vec<String>> {
        Ok(vec![
            format!("Running {}", run.name),
            "error: assertion failed: plan.steps.len() == 3".into(),
            "  left: 2, right: 3".into(),
            "Process exited with code 101".into(),
        ])
    }
    async fn list_changed_files(&self, id: &ChangeRequestId) -> ProviderResult<Vec<ChangedFile>> {
        Ok(diffs::files(&id.repo.path))
    }
    async fn get_file_content(
        &self,
        _id: &ChangeRequestId,
        path: &str,
        _rev: &str,
    ) -> ProviderResult<Option<String>> {
        Ok(diffs::content(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn lists_and_writes_in_memory() {
        let p = DemoProvider::new(ProviderKind::GitHub, GITHUB);
        let mine = p
            .list_change_requests(
                &Scope::InvolvingMe {
                    kind: ProviderKind::GitHub,
                    host: GITHUB.into(),
                },
                &Filter::default(),
            )
            .await
            .unwrap();
        assert!(!mine.is_empty() && mine.iter().all(involves_me));
        let id = mine[0].id.clone();
        let before = p.get_change_request(&id).await.unwrap().threads.len();
        p.add_comment(&id, "hi").await.unwrap();
        assert_eq!(
            p.get_change_request(&id).await.unwrap().threads.len(),
            before + 1
        );
    }
}
