use crate::*;
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("not authenticated for {0}")]
    Unauthenticated(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("http {status}: {body}")]
    Http { status: u16, body: String },
    #[error("network: {0}")]
    Network(String),
    #[error("decode: {0}")]
    Decode(String),
    /// The host asked us to slow down; nothing is sent to it before `until`.
    #[error("rate limited by {host} until {until}")]
    RateLimited { host: String, until: chrono::DateTime<chrono::Utc> },
}

pub type ProviderResult<T> = Result<T, ProviderError>;

/// One provider host (e.g. GitLab at gitlab.com) authenticated as one user.
#[async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn host(&self) -> &str;
    async fn whoami(&self) -> ProviderResult<User>;
    async fn list_change_requests(&self, scope: &Scope, filter: &Filter) -> ProviderResult<Vec<ChangeRequest>>;
    /// List several scopes. Providers may combine them into fewer requests (GitHub search does).
    async fn list_many(&self, scopes: Vec<Scope>, filter: &Filter) -> Vec<(Scope, ProviderResult<Vec<ChangeRequest>>)> {
        let mut out = Vec::with_capacity(scopes.len());
        for s in scopes {
            let r = self.list_change_requests(&s, filter).await;
            out.push((s, r));
        }
        out
    }
    /// Whether listed items lack reviewer states and checks until [`Provider::enrich`] fills them.
    fn lists_need_enrich(&self) -> bool {
        false
    }
    /// Fill reviewer states, checks and counts with extra requests. Called only for items whose
    /// `updated_at` changed since the cache (or whose checks were still running).
    /// Results are in input order.
    async fn enrich(&self, items: Vec<ChangeRequest>) -> Vec<ProviderResult<ChangeRequest>> {
        items.into_iter().map(Ok).collect()
    }
    async fn get_change_request(&self, id: &ChangeRequestId) -> ProviderResult<ChangeRequestDetail>;
    async fn list_repos(&self, owner: &str) -> ProviderResult<Vec<RepoRef>>;
    async fn add_comment(&self, id: &ChangeRequestId, body: &str) -> ProviderResult<Comment>;
    async fn reply_to_thread(&self, id: &ChangeRequestId, thread_id: &str, body: &str) -> ProviderResult<Comment>;
    async fn edit_comment(&self, id: &ChangeRequestId, comment_id: &str, body: &str) -> ProviderResult<Comment>;
    /// Start an inline review thread on a diff line (or range); returns the first comment.
    async fn add_line_comment(&self, id: &ChangeRequestId, position: &DiffPosition, body: &str) -> ProviderResult<Comment>;
    /// Mark an inline thread resolved or unresolved.
    async fn resolve_thread(&self, id: &ChangeRequestId, thread_id: &str, resolved: bool) -> ProviderResult<()>;
    /// Submit a review (approve, request changes, comment) or revoke an approval.
    async fn submit_review(&self, _id: &ChangeRequestId, action: ReviewAction, _body: Option<&str>) -> ProviderResult<()> {
        Err(ProviderError::Unsupported(format!("{} on {}", action.label(), self.kind())))
    }
    /// Change the title and/or description.
    async fn update_change_request(&self, _id: &ChangeRequestId, _title: Option<&str>, _body: Option<&str>) -> ProviderResult<()> {
        Err(ProviderError::Unsupported(format!("editing on {}", self.kind())))
    }
    /// Replace reviewers or assignees: `current` and `desired` are full lists (users carry ids on GitLab).
    async fn set_people(&self, _id: &ChangeRequestId, role: PeopleRole, _current: &[User], _desired: &[User]) -> ProviderResult<()> {
        Err(ProviderError::Unsupported(format!("{} on {}", role.label(), self.kind())))
    }
    /// Users that can be added as reviewers or assignees, matching `query`.
    async fn search_users(&self, _id: &ChangeRequestId, _query: &str) -> ProviderResult<Vec<User>> {
        Ok(vec![])
    }
    /// Mark as draft or ready for review. `title` is the current title (GitLab drafts by prefix),
    /// `node_id` the GraphQL id when known.
    async fn set_draft(&self, _id: &ChangeRequestId, _draft: bool, _title: &str, _node_id: Option<&str>) -> ProviderResult<()> {
        Err(ProviderError::Unsupported(format!("drafts on {}", self.kind())))
    }
    /// Close (`open == false`) or reopen.
    async fn set_open(&self, _id: &ChangeRequestId, _open: bool) -> ProviderResult<()> {
        Err(ProviderError::Unsupported(format!("closing on {}", self.kind())))
    }
    async fn merge(&self, _id: &ChangeRequestId, _opts: &MergeOptions) -> ProviderResult<()> {
        Err(ProviderError::Unsupported(format!("merging on {}", self.kind())))
    }
    /// Last lines of a failed check's log, or its annotations.
    async fn check_log(&self, _id: &ChangeRequestId, _run: &CheckRun) -> ProviderResult<Vec<String>> {
        Ok(vec![])
    }
    /// Re-run the failed jobs of `target` (see [`ChecksInfo::rerun_target`]).
    async fn rerun_failed(&self, _id: &ChangeRequestId, _target: &str) -> ProviderResult<()> {
        Err(ProviderError::Unsupported(format!("re-running checks on {}", self.kind())))
    }
    /// Add or remove my reaction on a comment (`subject` is its id) or, with `subject` empty, on the PR.
    async fn react(&self, _id: &ChangeRequestId, _subject: &str, _kind: ReactionKind, _add: bool) -> ProviderResult<()> {
        Err(ProviderError::Unsupported(format!("reactions on {}", self.kind())))
    }
    /// Mark a file viewed on the provider (GitHub only; others keep it locally).
    async fn set_file_viewed(&self, _id: &ChangeRequestId, _pr_node: Option<&str>, _path: &str, _viewed: bool) -> ProviderResult<()> {
        Err(ProviderError::Unsupported(format!("viewed files on {}", self.kind())))
    }
    /// Files changed by the PR/MR with parsed hunks.
    async fn list_changed_files(&self, id: &ChangeRequestId) -> ProviderResult<Vec<ChangedFile>>;
    /// Whole file at `rev` (a commit sha); None when it does not exist there.
    async fn get_file_content(&self, id: &ChangeRequestId, path: &str, rev: &str) -> ProviderResult<Option<String>>;
}
