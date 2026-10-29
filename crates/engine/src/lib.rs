//! Facade used by UIs (CLI now, desktop later). Owns the cache, credentials and provider clients.
//!
//! All async methods need a tokio runtime (reqwest and the CLI token helpers run on tokio).

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use futures_util::stream::{self, StreamExt};
use gv_auth::CredentialSource;
use gv_store::{CacheQuery, RepoOverride, Store};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub use gv_core::*;
pub use gv_store::{Project, Source};

/// Per-provider concurrency for per-repo refreshes.
const REPO_CONCURRENCY: usize = 4;

/// What to refresh.
#[derive(Debug, Clone)]
pub enum RefreshTarget {
    /// PRs/MRs involving the current user on every host with credentials.
    InvolvingMe,
    /// Every open PR/MR in a project's repos and groups.
    Project(i64),
}

/// Outcome of a refresh: how many items came back and which hosts/scopes failed.
#[derive(Debug, Default, Clone)]
pub struct RefreshReport {
    pub fetched: usize,
    pub pruned: usize,
    pub errors: Vec<(String, String)>,
}

/// Query over the cache.
#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    pub project: Option<i64>,
    pub filter: Filter,
    pub starred_only: bool,
    pub needs_you_only: bool,
}

type ProviderKey = (ProviderKind, String);

pub struct Engine {
    store: Store,
    creds: CredentialSource,
    providers: tokio::sync::Mutex<HashMap<ProviderKey, Arc<dyn Provider>>>,
    gitlab_hosts: Vec<String>,
    me_cache: Mutex<HashMap<ProviderKey, String>>,
}

/// Default cache location: `$GV_DB` or `~/Library/Application Support/git-viewer/cache.db`
/// (XDG data dir on other platforms).
pub fn default_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("GV_DB") {
        return PathBuf::from(p);
    }
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
    let base = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else {
        std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| home.join(".local/share"))
    };
    base.join("git-viewer/cache.db")
}

impl Engine {
    pub fn open(db: &Path) -> Result<Self> {
        let mut gitlab_hosts = gv_auth::glab_hosts();
        if !gitlab_hosts.iter().any(|h| h == "gitlab.com") {
            gitlab_hosts.push("gitlab.com".into());
        }
        Ok(Self {
            store: Store::open(db).with_context(|| format!("opening {}", db.display()))?,
            creds: CredentialSource::new(),
            providers: Default::default(),
            gitlab_hosts,
            me_cache: Default::default(),
        })
    }

    pub fn open_default() -> Result<Self> {
        Self::open(&default_db_path())
    }

    /// Direct access to the cache for advanced callers.
    pub fn store(&self) -> &Store {
        &self.store
    }

    // ---------- providers ----------

    /// Get (or build and memoize) an authenticated client for a host.
    pub async fn provider(&self, kind: ProviderKind, host: &str) -> Result<Arc<dyn Provider>> {
        let key = (kind, host.to_ascii_lowercase());
        let mut map = self.providers.lock().await;
        if let Some(p) = map.get(&key) {
            return Ok(p.clone());
        }
        let cred = self.creds.get(kind, host).await?;
        let p: Arc<dyn Provider> = match kind {
            ProviderKind::GitHub => Arc::new(gv_provider_github::GitHubProvider::new(host, cred.token)),
            ProviderKind::GitLab => Arc::new(gv_provider_gitlab::GitLabProvider::new(host, cred)),
            ProviderKind::Azure => Arc::new(gv_provider_azure::AzureProvider::new(host, cred.token)),
        };
        map.insert(key, p.clone());
        Ok(p)
    }

    pub async fn whoami(&self, kind: ProviderKind, host: &str) -> Result<User> {
        let u = self.provider(kind, host).await?.whoami().await?;
        self.remember_me(kind, host, &u.username);
        Ok(u)
    }

    fn remember_me(&self, kind: ProviderKind, host: &str, username: &str) {
        self.me_cache.lock().unwrap_or_else(|e| e.into_inner()).insert((kind, host.to_ascii_lowercase()), username.to_string());
        let _ = self.store.set_meta(&format!("me:{kind}:{host}"), username);
    }

    fn me(&self, kind: ProviderKind, host: &str) -> Option<String> {
        let key = (kind, host.to_ascii_lowercase());
        if let Some(u) = self.me_cache.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return Some(u.clone());
        }
        self.store.get_meta(&format!("me:{kind}:{host}")).ok().flatten()
    }

    /// Hosts that are queried for "involving me": github.com, glab hosts, plus hosts seen in projects.
    pub fn known_hosts(&self) -> Vec<(ProviderKind, String)> {
        let mut set: BTreeSet<(ProviderKind, String)> = BTreeSet::new();
        set.insert((ProviderKind::GitHub, "github.com".into()));
        for h in &self.gitlab_hosts {
            set.insert((ProviderKind::GitLab, h.clone()));
        }
        if let Ok(ps) = self.store.projects() {
            for p in ps {
                for r in self.store.project_repos(p.id).unwrap_or_default() {
                    if r.kind != ProviderKind::Azure {
                        set.insert((r.kind, r.host));
                    }
                }
            }
        }
        set.into_iter().collect()
    }

    // ---------- projects ----------

    pub fn projects(&self) -> Result<Vec<Project>> {
        Ok(self.store.projects()?)
    }

    pub fn project(&self, name: &str) -> Result<Project> {
        self.store.project_by_name(name)?.ok_or_else(|| anyhow!("no project named {name:?}"))
    }

    /// Create a project if missing and return it.
    pub fn ensure_project(&self, name: &str) -> Result<Project> {
        match self.store.project_by_name(name)? {
            Some(p) => Ok(p),
            None => Ok(self.store.create_project(name)?),
        }
    }

    /// Add a source to a project (creating the project if needed). Validates the source.
    pub fn add_source(&self, project: &str, source: Source) -> Result<Project> {
        match &source {
            Source::Folder { path, .. } if !path.is_dir() => bail!("{} is not a directory", path.display()),
            Source::Repo { url } if self.parse_repo_url(url).is_none() => bail!("cannot recognise repo URL {url}"),
            _ => {}
        }
        let p = self.ensure_project(project)?;
        self.store.add_source(p.id, &source)?;
        self.project(project)
    }

    pub fn remove_source(&self, project_id: i64, source: &Source) -> Result<()> {
        Ok(self.store.remove_source(project_id, source)?)
    }

    pub fn rename_project(&self, project_id: i64, name: &str) -> Result<()> {
        Ok(self.store.rename_project(project_id, name)?)
    }

    pub fn delete_project(&self, project_id: i64) -> Result<()> {
        Ok(self.store.delete_project(project_id)?)
    }

    pub fn set_repo_override(&self, repo: &RepoRef, o: RepoOverride) -> Result<()> {
        Ok(self.store.set_override(repo, &o)?)
    }

    pub fn repo_override(&self, repo: &RepoRef) -> Result<RepoOverride> {
        Ok(self.store.get_override(repo)?)
    }

    pub fn set_starred(&self, id: &ChangeRequestId, starred: bool) -> Result<()> {
        Ok(self.store.set_starred(id, starred)?)
    }

    /// Parse a git remote or web URL of a repository.
    pub fn parse_repo_url(&self, url: &str) -> Option<RepoRef> {
        let url = url.trim();
        // Strip web suffixes such as /-/merge_requests or /pulls.
        let cleaned = url.split("/-/").next().unwrap_or(url);
        gv_discovery::classify_remote(cleaned, &self.gitlab_hosts)
    }

    /// Resolve a project's sources to concrete repos (scan folders, expand groups) and cache the result.
    pub async fn resolve_project_repos(&self, project_id: i64) -> Result<Vec<RepoRef>> {
        let p = self.store.projects()?.into_iter().find(|p| p.id == project_id).ok_or_else(|| anyhow!("no project {project_id}"))?;
        let mut repos: BTreeSet<RepoRef> = BTreeSet::new();
        for s in &p.sources {
            match s {
                Source::Folder { path, depth } => {
                    let (path, depth, hosts) = (path.clone(), *depth, self.gitlab_hosts.clone());
                    let found = tokio::task::spawn_blocking(move || gv_discovery::discover_repos(&path, depth, &hosts)).await?;
                    repos.extend(found.into_iter().map(|(_, r)| r));
                }
                Source::Repo { url } => repos.extend(self.parse_repo_url(url)),
                Source::Group { provider, host, path } => {
                    let prov = self.provider(*provider, host).await?;
                    repos.extend(prov.list_repos(path).await?);
                }
            }
        }
        let repos: Vec<RepoRef> = repos.into_iter().collect();
        self.store.set_project_repos(project_id, &repos)?;
        Ok(repos)
    }

    // ---------- refresh ----------

    /// Fetch from providers into the cache. Providers run concurrently; failures are collected, not fatal.
    pub async fn refresh(&self, target: RefreshTarget) -> Result<RefreshReport> {
        let started = Utc::now();
        // (provider key) -> scopes to fetch
        let mut plan: BTreeMap<ProviderKey, Vec<Scope>> = BTreeMap::new();
        match &target {
            RefreshTarget::InvolvingMe => {
                for (k, h) in self.known_hosts() {
                    plan.entry((k, h.clone())).or_default().push(Scope::InvolvingMe { kind: k, host: h });
                }
            }
            RefreshTarget::Project(id) => {
                let p = self.store.projects()?.into_iter().find(|p| p.id == *id).ok_or_else(|| anyhow!("no project {id}"))?;
                let repos = self.resolve_project_repos(*id).await?;
                let groups: Vec<(ProviderKind, String, String)> = p
                    .sources
                    .iter()
                    .filter_map(|s| match s {
                        Source::Group { provider, host, path } => Some((*provider, host.clone(), path.clone())),
                        _ => None,
                    })
                    .collect();
                for (k, h, g) in &groups {
                    plan.entry((*k, h.clone())).or_default().push(Scope::Group { kind: *k, host: h.clone(), owner: g.clone() });
                }
                for r in repos {
                    let covered = groups.iter().any(|(k, h, g)| *k == r.kind && *h == r.host && r.path.starts_with(&format!("{g}/")));
                    if !covered {
                        plan.entry((r.kind, r.host.clone())).or_default().push(Scope::Repo(r));
                    }
                }
            }
        }
        let filter = Filter { involvement: Involvement::All, state: StateFilter::Active };
        let jobs = plan.into_iter().map(|((kind, host), scopes)| async move {
            let prov = match self.provider(kind, &host).await {
                Ok(p) => p,
                Err(e) => return vec![(format!("{kind}:{host}"), Err(e))],
            };
            if let Ok(me) = prov.whoami().await {
                self.remember_me(kind, &host, &me.username);
            }
            stream::iter(scopes)
                .map(|scope| {
                    let prov = prov.clone();
                    async move {
                        let label = format!("{scope:?}");
                        let r = prov.list_change_requests(&scope, &filter).await.map(|v| (scope, v)).map_err(anyhow::Error::from);
                        (label, r)
                    }
                })
                .buffer_unordered(REPO_CONCURRENCY)
                .collect::<Vec<_>>()
                .await
        });
        let results: Vec<_> = futures_util::future::join_all(jobs).await.into_iter().flatten().collect();

        let mut report = RefreshReport::default();
        for (label, r) in results {
            match r {
                Ok((scope, items)) => {
                    report.fetched += items.len();
                    self.store.upsert_change_requests(&items)?;
                    let seen: Vec<ChangeRequestId> = items.iter().map(|c| c.id.clone()).collect();
                    let me = match &scope {
                        Scope::InvolvingMe { kind, host } => self.me(*kind, host),
                        _ => None,
                    };
                    let in_scope = |cr: &ChangeRequest| scope_contains(&scope, cr, me.as_deref());
                    report.pruned += self.store.prune_stale(in_scope, &seen, started)?;
                }
                Err(e) => report.errors.push((label, format!("{e:#}"))),
            }
        }
        Ok(report)
    }

    // ---------- queries ----------

    /// Read change requests from the cache.
    pub fn list(&self, q: &ListQuery) -> Result<Vec<ChangeRequest>> {
        let repos = match q.project {
            Some(id) => Some(self.store.project_repos(id)?),
            None => None,
        };
        let me = self
            .known_hosts()
            .into_iter()
            .filter_map(|(k, h)| self.me(k, &h).map(|u| (k, h, u)))
            .collect();
        Ok(self.store.query(&CacheQuery {
            repos,
            filter: q.filter,
            me,
            starred_only: q.starred_only,
            needs_you_only: q.needs_you_only,
            include_hidden: false,
        })?)
    }

    /// Accept a web URL or a cache key (`gitlab:gitlab.com/g/p#12`).
    pub fn resolve_id(&self, s: &str) -> Result<ChangeRequestId> {
        let mut id = ChangeRequestId::from_web_url(s)
            .or_else(|| ChangeRequestId::parse_key(s))
            .ok_or_else(|| anyhow!("not a PR/MR URL or id: {s}"))?;
        // `/pull/` on a self-hosted GitLab-like host: trust our known GitLab hosts.
        if self.gitlab_hosts.contains(&id.repo.host) {
            id.repo.kind = ProviderKind::GitLab;
        }
        Ok(id)
    }

    /// Fetch full detail from the provider and cache it. Falls back to the cache on network errors when `allow_stale`.
    pub async fn detail(&self, id: &ChangeRequestId, allow_stale: bool) -> Result<ChangeRequestDetail> {
        let fetched = async {
            let prov = self.provider(id.repo.kind, &id.repo.host).await?;
            if let Ok(me) = prov.whoami().await {
                self.remember_me(id.repo.kind, &id.repo.host, &me.username);
            }
            Ok::<_, anyhow::Error>(prov.get_change_request(id).await?)
        }
        .await;
        match fetched {
            Ok(d) => {
                self.store.put_detail(&d)?;
                Ok(d)
            }
            Err(e) if allow_stale => self.store.get_detail(id)?.ok_or(e),
            Err(e) => Err(e),
        }
    }

    pub fn cached_detail(&self, id: &ChangeRequestId) -> Result<Option<ChangeRequestDetail>> {
        Ok(self.store.get_detail(id)?)
    }

    // ---------- comment actions ----------

    pub async fn add_comment(&self, id: &ChangeRequestId, body: &str) -> Result<Comment> {
        Ok(self.provider(id.repo.kind, &id.repo.host).await?.add_comment(id, body).await?)
    }

    pub async fn reply_to_thread(&self, id: &ChangeRequestId, thread_id: &str, body: &str) -> Result<Comment> {
        Ok(self.provider(id.repo.kind, &id.repo.host).await?.reply_to_thread(id, thread_id, body).await?)
    }

    pub async fn edit_comment(&self, id: &ChangeRequestId, comment_id: &str, body: &str) -> Result<Comment> {
        Ok(self.provider(id.repo.kind, &id.repo.host).await?.edit_comment(id, comment_id, body).await?)
    }
}

/// Whether a cached item belongs to the scope just refreshed (used for pruning).
fn scope_contains(scope: &Scope, cr: &ChangeRequest, me: Option<&str>) -> bool {
    let r = &cr.id.repo;
    match scope {
        Scope::Repo(x) => x == r,
        Scope::Group { kind, host, owner } => *kind == r.kind && *host == r.host && r.path.starts_with(&format!("{owner}/")),
        Scope::InvolvingMe { kind, host } => {
            let Some(me) = me else { return false };
            *kind == r.kind
                && *host == r.host
                && (cr.author.username.eq_ignore_ascii_case(me) || cr.reviewers.iter().any(|x| x.user.username.eq_ignore_ascii_case(me)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn projects_and_sources() {
        let dir = std::env::temp_dir().join(format!("gv-engine-{}", std::process::id()));
        let e = Engine::open(&dir.join("t.db")).unwrap();
        e.add_source("p", Source::Repo { url: "git@gitlab.com:g/sub/p.git".into() }).unwrap();
        assert!(e.add_source("p", Source::Repo { url: "nonsense".into() }).is_err());
        let p = e.project("p").unwrap();
        let repos = e.resolve_project_repos(p.id).await.unwrap();
        assert_eq!(repos, vec![RepoRef::new(ProviderKind::GitLab, "gitlab.com", "g/sub/p")]);
        assert_eq!(e.parse_repo_url("https://gitlab.com/g/p/-/merge_requests").unwrap().path, "g/p");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
