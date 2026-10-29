//! Azure DevOps provider. Phase 1 stub: every call returns `Unsupported`.

use async_trait::async_trait;
use gv_core::*;

pub struct AzureProvider {
    host: String,
}

impl AzureProvider {
    pub fn new(host: &str, _token: String) -> Self {
        Self { host: host.to_string() }
    }
}

fn unsupported<T>() -> ProviderResult<T> {
    Err(ProviderError::Unsupported("Azure DevOps is not implemented yet".into()))
}

#[async_trait]
impl Provider for AzureProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Azure
    }
    fn host(&self) -> &str {
        &self.host
    }
    async fn whoami(&self) -> ProviderResult<User> {
        unsupported()
    }
    async fn list_change_requests(&self, _: &Scope, _: &Filter) -> ProviderResult<Vec<ChangeRequest>> {
        unsupported()
    }
    async fn get_change_request(&self, _: &ChangeRequestId) -> ProviderResult<ChangeRequestDetail> {
        unsupported()
    }
    async fn list_repos(&self, _: &str) -> ProviderResult<Vec<RepoRef>> {
        unsupported()
    }
    async fn add_comment(&self, _: &ChangeRequestId, _: &str) -> ProviderResult<Comment> {
        unsupported()
    }
    async fn reply_to_thread(&self, _: &ChangeRequestId, _: &str, _: &str) -> ProviderResult<Comment> {
        unsupported()
    }
    async fn edit_comment(&self, _: &ChangeRequestId, _: &str, _: &str) -> ProviderResult<Comment> {
        unsupported()
    }
}
