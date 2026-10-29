//! Facade used by UIs (CLI now, desktop later). Owns the cache, credentials and provider clients.
//!
//! All async methods need a tokio runtime (reqwest and the CLI token helpers run on tokio).

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use gv_auth::CredentialSource;
use gv_store::{CacheQuery, Store};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod avatars;
mod bot_settings;

pub use avatars::small_avatar_url;
pub use bot_settings::bot_filter_from_settings;
pub use gv_core::*;
pub use gv_store::{Project, RepoOverride, Source};

/// Group repo listings are re-resolved at most this often (the group MR query covers new repos).
const GROUP_REPOS_TTL: chrono::Duration = chrono::Duration::hours(6);
/// A detail fetched this recently is served from the cache.
const DETAIL_FRESH: Duration = Duration::from_secs(60);

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
    /// Items whose reviewer states and checks were re-fetched (changed since the cache).
    pub enriched: usize,
    /// Set when a host is rate limited: nothing more is sent to it before this time.
    pub paused_until: Option<chrono::DateTime<Utc>>,
    /// The same target was already refreshing; this call did nothing.
    pub coalesced: bool,
}

/// Query over the cache.
#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    pub project: Option<i64>,
    pub filter: Filter,
    pub starred_only: bool,
    pub needs_you_only: bool,
    /// Bot PRs matching this filter are left out (see [`Engine::list_counted`]).
    pub bots: BotFilter,
}

type ProviderKey = (ProviderKind, String);

pub struct Engine {
    store: Store,
    creds: CredentialSource,
    providers: tokio::sync::Mutex<HashMap<ProviderKey, Arc<dyn Provider>>>,
    gitlab_hosts: Vec<String>,
    me_cache: Mutex<HashMap<ProviderKey, String>>,
    /// Refresh targets in flight; a second request for the same target is coalesced.
    in_flight: Mutex<BTreeSet<String>>,
    /// When each detail was last fetched from the provider.
    detail_fetched: Mutex<HashMap<ChangeRequestId, Instant>>,
}

/// Removes a refresh target from the in-flight set when dropped.
struct InFlight<'a>(&'a Mutex<BTreeSet<String>>, String);

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.1);
    }
}

fn later(a: Option<chrono::DateTime<Utc>>, b: chrono::DateTime<Utc>) -> Option<chrono::DateTime<Utc>> {
    Some(a.map_or(b, |a| a.max(b)))
}

/// App data dir: `~/Library/Application Support/Confluo` (XDG data dir on other platforms).
/// On first use, copies `settings.json` and `cache.db` from the old `git-viewer` dir,
/// which is left in place.
pub fn data_dir() -> PathBuf {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
    let base = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else {
        std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| home.join(".local/share"))
    };
    let dir = base.join("Confluo");
    migrate_legacy_dir(&base.join("git-viewer"), &dir);
    dir
}

/// Copy the files we own from `old` into `new` when `new` does not exist yet.
pub fn migrate_legacy_dir(old: &Path, new: &Path) {
    if new.exists() || !old.is_dir() {
        return;
    }
    if std::fs::create_dir_all(new).is_err() {
        return;
    }
    // SQLite sidecars too, so writes not yet checkpointed into cache.db survive.
    for f in ["settings.json", "cache.db", "cache.db-wal", "cache.db-shm"] {
        let src = old.join(f);
        if src.is_file() {
            let _ = std::fs::copy(&src, new.join(f));
        }
    }
}

/// Default cache location: `$GV_DB` or `<data_dir>/cache.db`.
pub fn default_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("GV_DB") {
        return PathBuf::from(p);
    }
    data_dir().join("cache.db")
}

#[cfg(test)]
mod data_dir_tests {
    use super::migrate_legacy_dir;

    #[test]
    fn migrates_once_and_keeps_old() {
        let root = std::env::temp_dir().join(format!("gv-mig-{}", std::process::id()));
        let (old, new) = (root.join("old"), root.join("new"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("settings.json"), "{}").unwrap();
        migrate_legacy_dir(&old, &new);
        assert_eq!(std::fs::read_to_string(new.join("settings.json")).unwrap(), "{}");
        assert!(old.join("settings.json").exists());
        std::fs::write(old.join("settings.json"), "changed").unwrap();
        migrate_legacy_dir(&old, &new);
        assert_eq!(std::fs::read_to_string(new.join("settings.json")).unwrap(), "{}");
        let _ = std::fs::remove_dir_all(&root);
    }
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
            in_flight: Default::default(),
            detail_fetched: Default::default(),
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

    /// Cached username on a host, if a previous call to the provider learned it.
    pub fn me(&self, kind: ProviderKind, host: &str) -> Option<String> {
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
        let _ = self.store.set_meta(&resolved_key(p.id), "");
        self.project(project)
    }

    pub fn remove_source(&self, project_id: i64, source: &Source) -> Result<()> {
        let _ = self.store.set_meta(&resolved_key(project_id), "");
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
        let _ = self.store.set_meta(&resolved_key(project_id), &Utc::now().to_rfc3339());
        Ok(repos)
    }

    /// The project's repos from the cache when resolved within `GROUP_REPOS_TTL`, else resolved now.
    /// Folder-only projects are always rescanned (local, no requests).
    async fn project_repos_cached(&self, p: &Project) -> Result<Vec<RepoRef>> {
        let has_group = p.sources.iter().any(|s| matches!(s, Source::Group { .. }));
        let resolved = self.store.get_meta(&resolved_key(p.id)).ok().flatten().and_then(|v| chrono::DateTime::parse_from_rfc3339(&v).ok());
        if has_group {
            if let Some(t) = resolved {
                if Utc::now() - t.with_timezone(&Utc) < GROUP_REPOS_TTL {
                    let repos = self.store.project_repos(p.id)?;
                    if !repos.is_empty() {
                        return Ok(repos);
                    }
                }
            }
        }
        self.resolve_project_repos(p.id).await
    }

    // ---------- refresh ----------

    /// Fetch from providers into the cache. Providers run concurrently; failures are collected, not fatal.
    ///
    /// Requests go through the per-host governor. Rate limited hosts are skipped and reported in
    /// `paused_until`, not as errors. A second call for a target that is already refreshing returns
    /// at once with `coalesced`. GitLab reviewer states and checks are re-fetched only for MRs whose
    /// `updated_at` moved (or whose checks were still running).
    pub async fn refresh(&self, target: RefreshTarget) -> Result<RefreshReport> {
        let key = refresh_key(&target);
        if !self.in_flight.lock().unwrap_or_else(|e| e.into_inner()).insert(key.clone()) {
            return Ok(RefreshReport { coalesced: true, ..Default::default() });
        }
        let _in_flight = InFlight(&self.in_flight, key);
        let started = Utc::now();
        let mut report = RefreshReport::default();
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
                let repos = match self.project_repos_cached(&p).await {
                    Ok(r) => r,
                    Err(e) => match e.downcast_ref::<ProviderError>() {
                        Some(ProviderError::RateLimited { until, .. }) => {
                            report.paused_until = later(report.paused_until, *until);
                            self.store.project_repos(*id)?
                        }
                        _ => return Err(e),
                    },
                };
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
        // Hosts in a rate limit pause are not contacted at all.
        plan.retain(|(_, host), _| match gv_http::governor(host).paused_until() {
            Some(t) => {
                report.paused_until = later(report.paused_until, t);
                false
            }
            None => true,
        });
        let filter = Filter { involvement: Involvement::All, state: StateFilter::Active };
        let jobs = plan.into_iter().map(|((kind, host), scopes)| async move {
            let prov = match self.provider(kind, &host).await {
                Ok(p) => p,
                Err(e) => return (None, vec![(host.clone(), None, Err(e))]),
            };
            if let Ok(me) = prov.whoami().await {
                self.remember_me(kind, &host, &me.username);
            }
            let results = prov
                .list_many(scopes, &filter)
                .await
                .into_iter()
                .map(|(scope, r)| (scope_label(&scope), Some(scope), r.map_err(anyhow::Error::from)))
                .collect::<Vec<_>>();
            (Some(prov), results)
        });
        let per_host: Vec<_> = futures_util::future::join_all(jobs).await;

        for (prov, results) in per_host {
            let mut ok: Vec<(Scope, Vec<ChangeRequest>)> = vec![];
            for (label, scope, r) in results {
                match (scope, r) {
                    (Some(scope), Ok(items)) => ok.push((scope, items)),
                    (_, Err(e)) => match e.downcast_ref::<ProviderError>() {
                        Some(ProviderError::RateLimited { until, .. }) => report.paused_until = later(report.paused_until, *until),
                        _ => report.errors.push((label, format!("{e:#}"))),
                    },
                    (None, Ok(_)) => {}
                }
            }
            if let Some(prov) = prov.filter(|p| p.lists_need_enrich()) {
                let (n, paused) = self.enrich_changed(prov.as_ref(), &mut ok).await;
                report.enriched += n;
                if let Some(t) = paused {
                    report.paused_until = later(report.paused_until, t);
                }
            }
            for (scope, items) in ok {
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
        }
        if (report.errors.is_empty() && report.paused_until.is_none()) || report.fetched > 0 {
            let _ = self.store.set_meta(&refresh_key(&target), &Utc::now().to_rfc3339());
        }
        Ok(report)
    }

    /// Carry cached reviewer states and checks onto unchanged items; enrich the rest.
    /// Returns how many were enriched and the end of a rate limit pause, if one hit.
    async fn enrich_changed(&self, prov: &dyn Provider, batches: &mut [(Scope, Vec<ChangeRequest>)]) -> (usize, Option<chrono::DateTime<Utc>>) {
        let me = self.me(prov.kind(), prov.host());
        // Hidden bot PRs are not worth requests; their detail view fetches what it needs.
        let bots = bot_filter_from_settings();
        let mut todo: Vec<ChangeRequest> = vec![];
        let mut todo_ids: BTreeSet<String> = BTreeSet::new();
        for (_, items) in batches.iter_mut() {
            for cr in items.iter_mut() {
                let cached = self.store.get_change_request(&cr.id).ok().flatten();
                let marker = self.store.get_meta(&enrich_key(&cr.id)).ok().flatten();
                let fresh = marker.as_deref() == Some(cr.updated_at.to_rfc3339().as_str());
                match cached {
                    Some(c) if fresh && c.checks.pending == 0 => carry_enrichment(cr, &c, me.as_deref()),
                    cached => {
                        // Until the new data arrives, stale enrichment beats none.
                        if let Some(c) = cached {
                            carry_enrichment(cr, &c, me.as_deref());
                        }
                        if !bots.hides(cr) && todo_ids.insert(cr.id.key()) {
                            todo.push(cr.clone());
                        }
                    }
                }
            }
        }
        if todo.is_empty() {
            return (0, None);
        }
        let mut paused = None;
        let mut done: HashMap<ChangeRequestId, ChangeRequest> = HashMap::new();
        for (orig, r) in todo.iter().zip(prov.enrich(todo.clone()).await) {
            match r {
                Ok(e) => {
                    let _ = self.store.set_meta(&enrich_key(&e.id), &orig.updated_at.to_rfc3339());
                    done.insert(e.id.clone(), e);
                }
                Err(ProviderError::RateLimited { until, .. }) => paused = later(paused, until),
                Err(_) => {}
            }
        }
        for (_, items) in batches.iter_mut() {
            for cr in items.iter_mut() {
                if let Some(e) = done.get(&cr.id) {
                    *cr = e.clone();
                }
            }
        }
        (done.len(), paused)
    }

    /// End of the latest rate limit pause across hosts, if any host is paused now.
    pub fn paused_until(&self) -> Option<chrono::DateTime<Utc>> {
        gv_http::paused().into_iter().map(|(_, t)| t).max()
    }

    /// Request counts per host and endpoint since start (for `GV_DEBUG_HTTP=1`).
    pub fn http_summary(&self) -> String {
        gv_http::stats_summary()
    }

    /// When `target` last refreshed successfully (persisted in the cache).
    pub fn last_refreshed(&self, target: &RefreshTarget) -> Option<chrono::DateTime<Utc>> {
        let v = self.store.get_meta(&refresh_key(target)).ok().flatten()?;
        chrono::DateTime::parse_from_rfc3339(&v).ok().map(|t| t.with_timezone(&Utc))
    }

    // ---------- queries ----------

    /// Read change requests from the cache.
    pub fn list(&self, q: &ListQuery) -> Result<Vec<ChangeRequest>> {
        Ok(self.list_counted(q)?.0)
    }

    /// Like [`Engine::list`], also returning how many bot PRs `q.bots` hid.
    pub fn list_counted(&self, q: &ListQuery) -> Result<(Vec<ChangeRequest>, usize)> {
        let repos = match q.project {
            Some(id) => Some(self.store.project_repos(id)?),
            None => None,
        };
        let me = self
            .known_hosts()
            .into_iter()
            .filter_map(|(k, h)| self.me(k, &h).map(|u| (k, h, u)))
            .collect();
        let items = self.store.query(&CacheQuery {
            repos,
            filter: q.filter,
            me,
            starred_only: q.starred_only,
            needs_you_only: q.needs_you_only,
            include_hidden: false,
        })?;
        let (hidden, shown): (Vec<_>, Vec<_>) = dedupe_by_id(items).into_iter().partition(|c| q.bots.hides(c));
        Ok((shown, hidden.len()))
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
    /// A detail fetched within `DETAIL_FRESH` is served from the cache; when the host is rate
    /// limited the cached detail is returned if there is one.
    pub async fn detail(&self, id: &ChangeRequestId, allow_stale: bool) -> Result<ChangeRequestDetail> {
        let recent = self.detail_fetched.lock().unwrap_or_else(|e| e.into_inner()).get(id).is_some_and(|t| t.elapsed() < DETAIL_FRESH);
        if recent {
            if let Some(d) = self.store.get_detail(id)? {
                return Ok(d);
            }
        }
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
                self.detail_fetched.lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), Instant::now());
                Ok(d)
            }
            Err(e) if allow_stale || matches!(e.downcast_ref::<ProviderError>(), Some(ProviderError::RateLimited { .. })) => {
                self.store.get_detail(id)?.ok_or(e)
            }
            Err(e) => Err(e),
        }
    }

    pub fn cached_detail(&self, id: &ChangeRequestId) -> Result<Option<ChangeRequestDetail>> {
        Ok(self.store.get_detail(id)?)
    }

    // ---------- changed files ----------

    /// Cached changed files, whatever head sha they were fetched at.
    pub fn cached_changed_files(&self, id: &ChangeRequestId) -> Option<Vec<ChangedFile>> {
        self.store.get_files(id).ok().flatten().map(|(_, f)| f)
    }

    /// Changed files at the current head. Cache-first: refetches only when the head sha moved.
    pub async fn changed_files(&self, id: &ChangeRequestId) -> Result<Vec<ChangedFile>> {
        let sha = match self.store.get_detail(id)?.and_then(|d| d.head_sha) {
            Some(s) => s,
            None => self.detail(id, true).await?.head_sha.unwrap_or_default(),
        };
        if let Some((cached_sha, files)) = self.store.get_files(id)? {
            if !sha.is_empty() && cached_sha == sha {
                return Ok(files);
            }
        }
        let files = self.provider(id.repo.kind, &id.repo.host).await?.list_changed_files(id).await?;
        self.store.put_files(id, &sha, &files)?;
        self.apply_totals(id, &files);
        Ok(files)
    }

    /// Fill +a -d and the file count into the cached summary (GitLab lists carry none).
    fn apply_totals(&self, id: &ChangeRequestId, files: &[ChangedFile]) {
        let Ok(Some(mut cr)) = self.store.get_change_request(id) else { return };
        let (a, d) = gv_core::totals(files);
        cr.additions = Some(a);
        cr.deletions = Some(d);
        cr.changed_files = Some(files.len() as u32);
        let _ = self.store.upsert_change_requests(&[cr]);
    }

    /// Whole file content at `rev` (usually the head sha).
    pub async fn file_content(&self, id: &ChangeRequestId, path: &str, rev: &str) -> Result<Option<String>> {
        Ok(self.provider(id.repo.kind, &id.repo.host).await?.get_file_content(id, path, rev).await?)
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

    /// Start an inline thread at `position`. Fills the head sha from the cached detail when missing.
    pub async fn add_line_comment(&self, id: &ChangeRequestId, position: &DiffPosition, body: &str) -> Result<Comment> {
        let mut pos = position.clone();
        if pos.head_sha.is_none() {
            pos.head_sha = self.store.get_detail(id)?.and_then(|d| d.head_sha);
        }
        if pos.other_line.is_none() {
            pos.other_line = self.cached_changed_files(id).and_then(|fs| context_partner(&fs, &pos));
        }
        Ok(self.provider(id.repo.kind, &id.repo.host).await?.add_line_comment(id, &pos, body).await?)
    }

    pub async fn resolve_thread(&self, id: &ChangeRequestId, thread_id: &str, resolved: bool) -> Result<()> {
        Ok(self.provider(id.repo.kind, &id.repo.host).await?.resolve_thread(id, thread_id, resolved).await?)
    }

    /// Inline threads of the cached detail placed on the cached diff (per path, side and line).
    pub fn inline_index(&self, id: &ChangeRequestId) -> InlineIndex {
        match (self.cached_detail(id).ok().flatten(), self.cached_changed_files(id)) {
            (Some(d), Some(f)) => index_inline_threads(&d.threads, &f),
            _ => InlineIndex::default(),
        }
    }
}

/// For a context line, the line number on the opposite side (GitLab anchors need both).
fn context_partner(files: &[ChangedFile], pos: &DiffPosition) -> Option<u32> {
    let f = files.iter().find(|f| f.new_path == pos.path || f.old_path == pos.path)?;
    let l = f.hunks.iter().flat_map(|h| &h.lines).find(|l| match pos.side {
        DiffSide::New => l.new_no == Some(pos.line),
        DiffSide::Old => l.old_no == Some(pos.line),
    })?;
    if l.kind != LineKind::Context {
        return None;
    }
    match pos.side {
        DiffSide::New => l.old_no,
        DiffSide::Old => l.new_no,
    }
}

fn resolved_key(project_id: i64) -> String {
    format!("resolved:project:{project_id}")
}

fn enrich_key(id: &ChangeRequestId) -> String {
    format!("enriched:{}", id.key())
}

/// Copy the enrichment (reviewer states, checks, counts) of a cached item onto a freshly listed one.
fn carry_enrichment(cr: &mut ChangeRequest, cached: &ChangeRequest, me: Option<&str>) {
    cr.reviewers = cached.reviewers.clone();
    cr.checks = cached.checks;
    cr.changed_files = cached.changed_files.or(cr.changed_files);
    cr.additions = cached.additions;
    cr.deletions = cached.deletions;
    if let Some(me) = me {
        cr.needs_you = compute_needs_you(cr, me);
    }
}

fn refresh_key(target: &RefreshTarget) -> String {
    match target {
        RefreshTarget::InvolvingMe => "refreshed:inbox".into(),
        RefreshTarget::Project(id) => format!("refreshed:project:{id}"),
    }
}

/// Short human label for a refresh scope, used in error reports.
fn scope_label(scope: &Scope) -> String {
    match scope {
        Scope::Repo(r) => format!("{}/{}", r.host, r.path),
        Scope::Group { host, owner, .. } => format!("{host}/{owner}"),
        Scope::InvolvingMe { host, .. } => host.clone(),
    }
}

/// Keep the first occurrence of each id (the list is sorted newest first).
fn dedupe_by_id(items: Vec<ChangeRequest>) -> Vec<ChangeRequest> {
    let mut seen = std::collections::HashSet::new();
    items.into_iter().filter(|c| seen.insert(c.id.clone())).collect()
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

    fn cr(n: u64, title: &str) -> ChangeRequest {
        ChangeRequest {
            id: ChangeRequestId { repo: RepoRef::new(ProviderKind::GitLab, "gitlab.com", "g/p"), number: n },
            title: title.into(),
            author: User::named("a"),
            state: ChangeRequestState::Open,
            source_branch: "a".into(),
            target_branch: "b".into(),
            reviewers: vec![],
            checks: CheckSummary::default(),
            labels: vec![],
            comments_count: 0,
            additions: None,
            deletions: None,
            changed_files: None,
            updated_at: Utc::now(),
            created_at: Utc::now(),
            web_url: String::new(),
            needs_you: false,
        }
    }

    #[test]
    fn dedupes_by_id_keeping_first() {
        let v = dedupe_by_id(vec![cr(1, "new"), cr(2, "other"), cr(1, "old")]);
        assert_eq!(v.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), vec!["new", "other"]);
    }

    #[test]
    fn list_counted_hides_bots() {
        let dir = std::env::temp_dir().join(format!("gv-engine-bots-{}", std::process::id()));
        let e = Engine::open(&dir.join("t.db")).unwrap();
        let mut bot = cr(3, "Update react");
        bot.author = User::named("acme-bot");
        bot.source_branch = "renovate/react".into();
        e.store().upsert_change_requests(&[cr(1, "x"), bot]).unwrap();
        let (v, hidden) = e.list_counted(&ListQuery::default()).unwrap();
        assert_eq!((v.len(), hidden), (1, 1));
        let (v, hidden) = e.list_counted(&ListQuery { bots: BotFilter::off(), ..Default::default() }).unwrap();
        assert_eq!((v.len(), hidden), (2, 0));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn carries_enrichment_and_recomputes_needs_you() {
        let mut cached = cr(1, "x");
        cached.reviewers = vec![Review { user: User::named("me"), state: ReviewState::Pending }];
        cached.checks = CheckSummary { total: 2, passed: 2, failed: 0, pending: 0 };
        let mut fresh = cr(1, "x");
        carry_enrichment(&mut fresh, &cached, Some("me"));
        assert_eq!(fresh.checks.total, 2);
        assert!(fresh.needs_you);
    }

    #[test]
    fn list_has_unique_ids() {
        let dir = std::env::temp_dir().join(format!("gv-engine-list-{}", std::process::id()));
        let e = Engine::open(&dir.join("t.db")).unwrap();
        e.store().upsert_change_requests(&[cr(1, "x"), cr(1, "y"), cr(2, "z")]).unwrap();
        let v = e.list(&ListQuery::default()).unwrap();
        let ids: std::collections::HashSet<_> = v.iter().map(|c| c.id.clone()).collect();
        assert_eq!(ids.len(), v.len());
        assert_eq!(v.len(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
