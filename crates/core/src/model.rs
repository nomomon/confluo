use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    GitHub,
    GitLab,
    Azure,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::GitHub => "github",
            ProviderKind::GitLab => "gitlab",
            ProviderKind::Azure => "azure",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "github" | "gh" => Some(ProviderKind::GitHub),
            "gitlab" | "gl" => Some(ProviderKind::GitLab),
            "azure" | "ado" | "azuredevops" => Some(ProviderKind::Azure),
            _ => None,
        }
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A repository on a specific provider host, e.g. (GitLab, gitlab.com, "group/sub/project").
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RepoRef {
    pub kind: ProviderKind,
    pub host: String,
    pub path: String,
}

impl RepoRef {
    pub fn new(kind: ProviderKind, host: impl Into<String>, path: impl Into<String>) -> Self {
        Self { kind, host: host.into().to_ascii_lowercase(), path: path.into() }
    }
    pub fn web_url(&self) -> String {
        format!("https://{}/{}", self.host, self.path)
    }
}

impl fmt::Display for RepoRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}/{}", self.kind, self.host, self.path)
    }
}

/// Identity of a PR/MR: repo + number (GitHub number / GitLab iid / Azure id).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChangeRequestId {
    pub repo: RepoRef,
    pub number: u64,
}

impl ChangeRequestId {
    /// Stable string key, e.g. `gitlab:gitlab.com/group/proj!12`.
    pub fn key(&self) -> String {
        format!("{}#{}", self.repo, self.number)
    }

    /// Parse a key produced by [`ChangeRequestId::key`].
    pub fn parse_key(s: &str) -> Option<Self> {
        let (repo, num) = s.rsplit_once('#')?;
        let (kind, rest) = repo.split_once(':')?;
        let (host, path) = rest.split_once('/')?;
        Some(Self {
            repo: RepoRef::new(ProviderKind::parse(kind)?, host, path),
            number: num.parse().ok()?,
        })
    }

    /// Parse a web URL of a PR/MR on GitHub, GitLab or Azure DevOps.
    pub fn from_web_url(url: &str) -> Option<Self> {
        let rest = url.trim().strip_prefix("https://").or_else(|| url.trim().strip_prefix("http://"))?;
        let rest = rest.split(['?', '#']).next()?.trim_end_matches('/');
        let (host, path) = rest.split_once('/')?;
        let host = host.to_ascii_lowercase();
        if let Some((repo, tail)) = path.split_once("/-/merge_requests/") {
            let n = tail.split('/').next()?.parse().ok()?;
            return Some(Self { repo: RepoRef::new(ProviderKind::GitLab, host, repo), number: n });
        }
        if let Some((repo, tail)) = path.split_once("/_git/") {
            // dev.azure.com/org/project/_git/repo/pullrequest/12
            let (name, tail) = tail.split_once("/pullrequest/")?;
            let n = tail.split('/').next()?.parse().ok()?;
            return Some(Self {
                repo: RepoRef::new(ProviderKind::Azure, host, format!("{repo}/{name}")),
                number: n,
            });
        }
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 4 && parts[2] == "pull" {
            let kind = if host.contains("gitlab") { ProviderKind::GitLab } else { ProviderKind::GitHub };
            return Some(Self {
                repo: RepoRef::new(kind, host, format!("{}/{}", parts[0], parts[1])),
                number: parts[3].parse().ok()?,
            });
        }
        None
    }
}

impl fmt::Display for ChangeRequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sep = if self.repo.kind == ProviderKind::GitLab { '!' } else { '#' };
        write!(f, "{}{}{}", self.repo.path, sep, self.number)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    /// The provider marks this account as a bot (GitHub `Bot`, GitLab `bot: true`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bot: bool,
}

impl User {
    pub fn named(username: impl Into<String>) -> Self {
        Self { username: username.into(), ..Default::default() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeRequestState {
    Open,
    Draft,
    Merged,
    Closed,
}

impl ChangeRequestState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Open | Self::Draft)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    Pending,
    Commented,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    pub user: User,
    pub state: ReviewState,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub pending: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeRequest {
    pub id: ChangeRequestId,
    pub title: String,
    pub author: User,
    pub state: ChangeRequestState,
    pub source_branch: String,
    pub target_branch: String,
    pub reviewers: Vec<Review>,
    pub checks: CheckSummary,
    pub labels: Vec<String>,
    pub comments_count: u32,
    pub additions: Option<u32>,
    pub deletions: Option<u32>,
    pub changed_files: Option<u32>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub web_url: String,
    /// Whether the viewer is expected to act. See [`crate::compute_needs_you`].
    pub needs_you: bool,
}

impl ChangeRequest {
    pub fn repo(&self) -> &RepoRef {
        &self.id.repo
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlinePosition {
    pub path: String,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: User,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub position: Option<InlinePosition>,
}

/// Which side of a diff a line belongs to: the base (old) or the head (new) file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiffSide {
    Old,
    #[default]
    New,
}

impl DiffSide {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "old" | "left" | "base" => Some(Self::Old),
            "new" | "right" | "head" => Some(Self::New),
            _ => None,
        }
    }
}

/// Where an inline review comment is anchored in a diff.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiffPosition {
    /// Path on the anchored side (new path for New, old path for Old).
    pub path: String,
    /// Path before a rename, when it differs.
    pub old_path: Option<String>,
    pub side: DiffSide,
    /// Last line of the range (the anchor line).
    pub line: u32,
    /// First line of a multi-line range on the same side.
    pub start_line: Option<u32>,
    /// For context lines: the line number on the other side (GitLab needs both).
    pub other_line: Option<u32>,
    pub head_sha: Option<String>,
    pub base_sha: Option<String>,
    pub start_sha: Option<String>,
}

impl DiffPosition {
    pub fn new(path: impl Into<String>, side: DiffSide, line: u32) -> Self {
        Self { path: path.into(), side, line, ..Default::default() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub resolved: Option<bool>,
    pub comments: Vec<Comment>,
    /// Anchor of an inline review thread; None for conversation comments.
    #[serde(default)]
    pub position: Option<DiffPosition>,
    /// The anchored line is no longer part of the current diff.
    #[serde(default)]
    pub outdated: bool,
}

impl Thread {
    pub fn conversation(id: impl Into<String>, comments: Vec<Comment>) -> Self {
        Self { id: id.into(), resolved: None, comments, position: None, outdated: false }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeRequestDetail {
    pub summary: ChangeRequest,
    /// Markdown body.
    pub description: String,
    pub threads: Vec<Thread>,
    pub head_sha: Option<String>,
    /// Review actions I can take and my current verdict.
    #[serde(default)]
    pub review: crate::ReviewInfo,
    /// Opened, reviews, merged: the non-comment items of the timeline.
    #[serde(default)]
    pub events: Vec<crate::TimelineEvent>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Involvement {
    Authored,
    Reviewing,
    #[default]
    All,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateFilter {
    /// Open and draft.
    #[default]
    Active,
    Merged,
    Closed,
    Any,
}

impl StateFilter {
    pub fn matches(&self, s: ChangeRequestState) -> bool {
        match self {
            StateFilter::Active => s.is_active(),
            StateFilter::Merged => s == ChangeRequestState::Merged,
            StateFilter::Closed => s == ChangeRequestState::Closed,
            StateFilter::Any => true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filter {
    pub involvement: Involvement,
    pub state: StateFilter,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    Repo(RepoRef),
    /// GitHub org/user or GitLab group (path may be nested).
    Group { kind: ProviderKind, host: String, owner: String },
    /// Everything on a host that involves the current user.
    InvolvingMe { kind: ProviderKind, host: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_urls() {
        let g = ChangeRequestId::from_web_url("https://github.com/nomomon/zeron/pull/4").unwrap();
        assert_eq!(g.repo, RepoRef::new(ProviderKind::GitHub, "github.com", "nomomon/zeron"));
        assert_eq!(g.number, 4);
        let l = ChangeRequestId::from_web_url("https://gitlab.com/a/b/c/-/merge_requests/17/diffs?x=1").unwrap();
        assert_eq!(l.repo.path, "a/b/c");
        assert_eq!(l.repo.kind, ProviderKind::GitLab);
        assert_eq!(l.number, 17);
        let a = ChangeRequestId::from_web_url("https://dev.azure.com/org/proj/_git/repo/pullrequest/9").unwrap();
        assert_eq!(a.repo.path, "org/proj/repo");
        assert_eq!(a.repo.kind, ProviderKind::Azure);
    }

    #[test]
    fn key_roundtrip() {
        let id = ChangeRequestId { repo: RepoRef::new(ProviderKind::GitLab, "gitlab.com", "a/b"), number: 3 };
        assert_eq!(ChangeRequestId::parse_key(&id.key()), Some(id));
    }
}
