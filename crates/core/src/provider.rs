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
}

pub type ProviderResult<T> = Result<T, ProviderError>;

/// One provider host (e.g. GitLab at gitlab.com) authenticated as one user.
#[async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn host(&self) -> &str;
    async fn whoami(&self) -> ProviderResult<User>;
    async fn list_change_requests(&self, scope: &Scope, filter: &Filter) -> ProviderResult<Vec<ChangeRequest>>;
    async fn get_change_request(&self, id: &ChangeRequestId) -> ProviderResult<ChangeRequestDetail>;
    async fn list_repos(&self, owner: &str) -> ProviderResult<Vec<RepoRef>>;
    async fn add_comment(&self, id: &ChangeRequestId, body: &str) -> ProviderResult<Comment>;
    async fn reply_to_thread(&self, id: &ChangeRequestId, thread_id: &str, body: &str) -> ProviderResult<Comment>;
    async fn edit_comment(&self, id: &ChangeRequestId, comment_id: &str, body: &str) -> ProviderResult<Comment>;
    /// Start an inline review thread on a diff line (or range); returns the first comment.
    async fn add_line_comment(&self, id: &ChangeRequestId, position: &DiffPosition, body: &str) -> ProviderResult<Comment>;
    /// Mark an inline thread resolved or unresolved.
    async fn resolve_thread(&self, id: &ChangeRequestId, thread_id: &str, resolved: bool) -> ProviderResult<()>;
    /// Files changed by the PR/MR with parsed hunks.
    async fn list_changed_files(&self, id: &ChangeRequestId) -> ProviderResult<Vec<ChangedFile>>;
    /// Whole file at `rev` (a commit sha); None when it does not exist there.
    async fn get_file_content(&self, id: &ChangeRequestId, path: &str, rev: &str) -> ProviderResult<Option<String>>;
}
