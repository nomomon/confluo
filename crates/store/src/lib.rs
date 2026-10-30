//! SQLite cache: projects and their sources, repo overrides, cached change requests, stars.

use chrono::{DateTime, Utc};
use gv_core::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    /// Local folder scanned for git repos.
    Folder { path: PathBuf, depth: usize },
    /// A single repo by remote/web URL.
    Repo { url: String },
    /// A GitHub owner or GitLab group (nested path allowed).
    Group {
        provider: ProviderKind,
        host: String,
        path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoOverride {
    pub alias: Option<String>,
    pub hidden: bool,
}

/// Query over cached change requests.
#[derive(Debug, Clone, Default)]
pub struct CacheQuery {
    /// Restrict to these repos (e.g. a project's repos). None = all.
    pub repos: Option<Vec<RepoRef>>,
    pub filter: Filter,
    /// Username per (kind, host) used for authored/reviewing filtering.
    pub me: Vec<(ProviderKind, String, String)>,
    pub starred_only: bool,
    pub needs_you_only: bool,
    pub include_hidden: bool,
}

mod viewed;

pub struct Store {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS projects (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);
CREATE TABLE IF NOT EXISTS project_sources (
  id INTEGER PRIMARY KEY, project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE, data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS project_repos (
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE, repo TEXT NOT NULL, data TEXT NOT NULL,
  PRIMARY KEY (project_id, repo));
CREATE TABLE IF NOT EXISTS repo_overrides (repo TEXT PRIMARY KEY, alias TEXT, hidden INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS change_requests (
  key TEXT PRIMARY KEY, repo TEXT NOT NULL, updated_at TEXT NOT NULL, data TEXT NOT NULL, fetched_at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS cr_repo ON change_requests(repo);
CREATE TABLE IF NOT EXISTS details (key TEXT PRIMARY KEY, data TEXT NOT NULL, fetched_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS diffs (key TEXT PRIMARY KEY, sha TEXT NOT NULL, data TEXT NOT NULL, fetched_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS starred (key TEXT PRIMARY KEY);
CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS viewed (pr TEXT NOT NULL, path TEXT NOT NULL, fp TEXT NOT NULL, PRIMARY KEY (pr, path));
"#;

impl Store {
    pub fn open(path: &Path) -> StoreResult<Self> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| StoreError::Invalid(e.to_string()))?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn in_memory() -> StoreResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> StoreResult<Self> {
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn c(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    // ---- projects ----

    pub fn create_project(&self, name: &str) -> StoreResult<Project> {
        let c = self.c();
        c.execute("INSERT INTO projects(name) VALUES (?1)", [name])?;
        Ok(Project {
            id: c.last_insert_rowid(),
            name: name.into(),
            sources: vec![],
        })
    }

    pub fn add_source(&self, project_id: i64, source: &Source) -> StoreResult<()> {
        let data = serde_json::to_string(source)?;
        let c = self.c();
        let exists: bool = c
            .query_row(
                "SELECT 1 FROM project_sources WHERE project_id=?1 AND data=?2",
                params![project_id, data],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !exists {
            c.execute(
                "INSERT INTO project_sources(project_id, data) VALUES (?1, ?2)",
                params![project_id, data],
            )?;
        }
        Ok(())
    }

    pub fn remove_source(&self, project_id: i64, source: &Source) -> StoreResult<()> {
        let data = serde_json::to_string(source)?;
        self.c().execute(
            "DELETE FROM project_sources WHERE project_id=?1 AND data=?2",
            params![project_id, data],
        )?;
        Ok(())
    }

    pub fn delete_project(&self, id: i64) -> StoreResult<()> {
        self.c().execute("DELETE FROM projects WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn rename_project(&self, id: i64, name: &str) -> StoreResult<()> {
        self.c()
            .execute("UPDATE projects SET name=?2 WHERE id=?1", params![id, name])?;
        Ok(())
    }

    pub fn projects(&self) -> StoreResult<Vec<Project>> {
        let c = self.c();
        let mut st = c.prepare("SELECT id, name FROM projects ORDER BY name")?;
        let rows: Vec<(i64, String)> = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let mut src =
            c.prepare("SELECT data FROM project_sources WHERE project_id=?1 ORDER BY id")?;
        let mut out = vec![];
        for (id, name) in rows {
            let sources = src
                .query_map([id], |r| r.get::<_, String>(0))?
                .filter_map(|s| s.ok().and_then(|s| serde_json::from_str(&s).ok()))
                .collect();
            out.push(Project { id, name, sources });
        }
        Ok(out)
    }

    pub fn project_by_name(&self, name: &str) -> StoreResult<Option<Project>> {
        Ok(self
            .projects()?
            .into_iter()
            .find(|p| p.name.eq_ignore_ascii_case(name)))
    }

    /// Replace the resolved repo list of a project.
    pub fn set_project_repos(&self, project_id: i64, repos: &[RepoRef]) -> StoreResult<()> {
        let mut c = self.c();
        let tx = c.transaction()?;
        tx.execute(
            "DELETE FROM project_repos WHERE project_id=?1",
            [project_id],
        )?;
        for r in repos {
            tx.execute(
                "INSERT OR IGNORE INTO project_repos(project_id, repo, data) VALUES (?1, ?2, ?3)",
                params![project_id, r.to_string(), serde_json::to_string(r)?],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn project_repos(&self, project_id: i64) -> StoreResult<Vec<RepoRef>> {
        let c = self.c();
        let mut st =
            c.prepare("SELECT data FROM project_repos WHERE project_id=?1 ORDER BY repo")?;
        let v = st
            .query_map([project_id], |r| r.get::<_, String>(0))?
            .filter_map(|s| s.ok().and_then(|s| serde_json::from_str(&s).ok()))
            .collect();
        Ok(v)
    }

    // ---- overrides ----

    pub fn set_override(&self, repo: &RepoRef, o: &RepoOverride) -> StoreResult<()> {
        self.c().execute(
            "INSERT INTO repo_overrides(repo, alias, hidden) VALUES (?1, ?2, ?3)
             ON CONFLICT(repo) DO UPDATE SET alias=excluded.alias, hidden=excluded.hidden",
            params![repo.to_string(), o.alias, o.hidden],
        )?;
        Ok(())
    }

    pub fn get_override(&self, repo: &RepoRef) -> StoreResult<RepoOverride> {
        Ok(self
            .c()
            .query_row(
                "SELECT alias, hidden FROM repo_overrides WHERE repo=?1",
                [repo.to_string()],
                |r| {
                    Ok(RepoOverride {
                        alias: r.get(0)?,
                        hidden: r.get(1)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default())
    }

    fn hidden_repos(&self) -> StoreResult<Vec<String>> {
        let c = self.c();
        let mut st = c.prepare("SELECT repo FROM repo_overrides WHERE hidden=1")?;
        let v = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        Ok(v)
    }

    // ---- change requests ----

    /// Upsert fetched change requests.
    pub fn upsert_change_requests(&self, items: &[ChangeRequest]) -> StoreResult<()> {
        let now = Utc::now().to_rfc3339();
        let mut c = self.c();
        let tx = c.transaction()?;
        for cr in items {
            tx.execute(
                "INSERT INTO change_requests(key, repo, updated_at, data, fetched_at) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(key) DO UPDATE SET repo=excluded.repo, updated_at=excluded.updated_at, data=excluded.data, fetched_at=excluded.fetched_at",
                params![cr.id.key(), cr.id.repo.to_string(), cr.updated_at.to_rfc3339(), serde_json::to_string(cr)?, now],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Remove cached active items matching `in_scope` that were not refreshed since `fetched_before`,
    /// so PRs closed elsewhere (or no longer involving you) do not linger as open.
    pub fn prune_stale(
        &self,
        in_scope: impl Fn(&ChangeRequest) -> bool,
        seen: &[ChangeRequestId],
        fetched_before: DateTime<Utc>,
    ) -> StoreResult<usize> {
        let keys: std::collections::HashSet<String> = seen.iter().map(|i| i.key()).collect();
        let c = self.c();
        let mut st = c.prepare("SELECT key, data FROM change_requests WHERE fetched_at < ?1")?;
        let stale: Vec<String> = st
            .query_map([fetched_before.to_rfc3339()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .filter(|(k, d)| {
                !keys.contains(k)
                    && serde_json::from_str::<ChangeRequest>(d)
                        .map(|cr| cr.state.is_active() && in_scope(&cr))
                        .unwrap_or(true)
            })
            .map(|(k, _)| k)
            .collect();
        for k in &stale {
            c.execute("DELETE FROM change_requests WHERE key=?1", [k])?;
        }
        Ok(stale.len())
    }

    pub fn get_change_request(&self, id: &ChangeRequestId) -> StoreResult<Option<ChangeRequest>> {
        let d: Option<String> = self
            .c()
            .query_row(
                "SELECT data FROM change_requests WHERE key=?1",
                [id.key()],
                |r| r.get(0),
            )
            .optional()?;
        Ok(match d {
            Some(d) => Some(serde_json::from_str(&d)?),
            None => None,
        })
    }

    pub fn query(&self, q: &CacheQuery) -> StoreResult<Vec<ChangeRequest>> {
        let hidden = if q.include_hidden {
            vec![]
        } else {
            self.hidden_repos()?
        };
        let starred = self.starred()?;
        let c = self.c();
        let mut st = c.prepare("SELECT data FROM change_requests ORDER BY updated_at DESC")?;
        let all: Vec<ChangeRequest> = st
            .query_map([], |r| r.get::<_, String>(0))?
            .filter_map(|s| s.ok().and_then(|s| serde_json::from_str(&s).ok()))
            .collect();
        let me_for = |cr: &ChangeRequest| {
            q.me.iter()
                .find(|(k, h, _)| *k == cr.id.repo.kind && h.eq_ignore_ascii_case(&cr.id.repo.host))
                .map(|(_, _, u)| u.as_str())
        };
        Ok(all
            .into_iter()
            .filter(|cr| q.filter.state.matches(cr.state))
            .filter(|cr| q.repos.as_ref().is_none_or(|rs| rs.contains(&cr.id.repo)))
            .filter(|cr| !hidden.contains(&cr.id.repo.to_string()))
            .filter(|cr| !q.starred_only || starred.contains(&cr.id))
            .filter(|cr| !q.needs_you_only || cr.needs_you)
            .filter(|cr| match (q.filter.involvement, me_for(cr)) {
                (Involvement::All, _) => true,
                (_, None) => false,
                (Involvement::Authored, Some(me)) => cr.author.username.eq_ignore_ascii_case(me),
                (Involvement::Reviewing, Some(me)) => {
                    cr.reviewers
                        .iter()
                        .any(|r| r.user.username.eq_ignore_ascii_case(me))
                        && !cr.author.username.eq_ignore_ascii_case(me)
                }
            })
            .collect())
    }

    // ---- details ----

    pub fn put_detail(&self, d: &ChangeRequestDetail) -> StoreResult<()> {
        self.upsert_change_requests(std::slice::from_ref(&d.summary))?;
        self.c().execute(
            "INSERT OR REPLACE INTO details(key, data, fetched_at) VALUES (?1, ?2, ?3)",
            params![
                d.summary.id.key(),
                serde_json::to_string(d)?,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn get_detail(&self, id: &ChangeRequestId) -> StoreResult<Option<ChangeRequestDetail>> {
        let d: Option<String> = self
            .c()
            .query_row("SELECT data FROM details WHERE key=?1", [id.key()], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(match d {
            Some(d) => Some(serde_json::from_str(&d)?),
            None => None,
        })
    }

    // ---- changed files ----

    pub fn put_files(
        &self,
        id: &ChangeRequestId,
        sha: &str,
        files: &[ChangedFile],
    ) -> StoreResult<()> {
        self.c().execute(
            "INSERT OR REPLACE INTO diffs(key, sha, data, fetched_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                id.key(),
                sha,
                serde_json::to_string(files)?,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    /// Cached files and the head sha they were fetched at.
    pub fn get_files(
        &self,
        id: &ChangeRequestId,
    ) -> StoreResult<Option<(String, Vec<ChangedFile>)>> {
        let row: Option<(String, String)> = self
            .c()
            .query_row(
                "SELECT sha, data FROM diffs WHERE key=?1",
                [id.key()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        Ok(match row {
            Some((sha, d)) => Some((sha, serde_json::from_str(&d)?)),
            None => None,
        })
    }

    // ---- stars ----

    pub fn set_starred(&self, id: &ChangeRequestId, starred: bool) -> StoreResult<()> {
        if starred {
            self.c()
                .execute("INSERT OR IGNORE INTO starred(key) VALUES (?1)", [id.key()])?;
        } else {
            self.c()
                .execute("DELETE FROM starred WHERE key=?1", [id.key()])?;
        }
        Ok(())
    }

    pub fn starred(&self) -> StoreResult<Vec<ChangeRequestId>> {
        let c = self.c();
        let mut st = c.prepare("SELECT key FROM starred")?;
        let v = st
            .query_map([], |r| r.get::<_, String>(0))?
            .filter_map(|k| k.ok().and_then(|k| ChangeRequestId::parse_key(&k)))
            .collect();
        Ok(v)
    }

    // ---- meta (small key/value, e.g. cached usernames) ----

    pub fn set_meta(&self, k: &str, v: &str) -> StoreResult<()> {
        self.c().execute(
            "INSERT OR REPLACE INTO meta(k, v) VALUES (?1, ?2)",
            params![k, v],
        )?;
        Ok(())
    }

    pub fn get_meta(&self, k: &str) -> StoreResult<Option<String>> {
        Ok(self
            .c()
            .query_row("SELECT v FROM meta WHERE k=?1", [k], |r| r.get(0))
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cr(n: u64, author: &str, state: ChangeRequestState) -> ChangeRequest {
        ChangeRequest {
            id: ChangeRequestId {
                repo: RepoRef::new(ProviderKind::GitLab, "gitlab.com", "g/p"),
                number: n,
            },
            title: format!("t{n}"),
            author: User::named(author),
            state,
            source_branch: "a".into(),
            target_branch: "b".into(),
            reviewers: vec![Review {
                user: User::named("me"),
                state: ReviewState::Pending,
            }],
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
            signals: Default::default(),
        }
    }

    #[test]
    fn files_roundtrip() {
        let s = Store::in_memory().unwrap();
        let id = cr(1, "me", ChangeRequestState::Open).id;
        assert!(s.get_files(&id).unwrap().is_none());
        let f = ChangedFile::from_patch("a", "a", FileStatus::Modified, "@@ -1 +1 @@\n-x\n+y\n");
        s.put_files(&id, "abc", std::slice::from_ref(&f)).unwrap();
        assert_eq!(s.get_files(&id).unwrap(), Some(("abc".into(), vec![f])));
    }

    #[test]
    fn projects_roundtrip() {
        let s = Store::in_memory().unwrap();
        let p = s.create_project("work").unwrap();
        let src = Source::Group {
            provider: ProviderKind::GitLab,
            host: "gitlab.com".into(),
            path: "a/b".into(),
        };
        s.add_source(p.id, &src).unwrap();
        s.add_source(p.id, &src).unwrap();
        s.add_source(
            p.id,
            &Source::Folder {
                path: "/x".into(),
                depth: 2,
            },
        )
        .unwrap();
        let ps = s.projects().unwrap();
        assert_eq!(ps[0].sources.len(), 2);
        s.delete_project(p.id).unwrap();
        assert!(s.projects().unwrap().is_empty());
    }

    #[test]
    fn query_filters() {
        let s = Store::in_memory().unwrap();
        s.upsert_change_requests(&[
            cr(1, "me", ChangeRequestState::Open),
            cr(2, "bob", ChangeRequestState::Open),
            cr(3, "me", ChangeRequestState::Merged),
        ])
        .unwrap();
        let me = vec![(
            ProviderKind::GitLab,
            "gitlab.com".to_string(),
            "me".to_string(),
        )];
        let q = |inv, state| CacheQuery {
            filter: Filter {
                involvement: inv,
                state,
            },
            me: me.clone(),
            ..Default::default()
        };
        assert_eq!(
            s.query(&q(Involvement::All, StateFilter::Active))
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            s.query(&q(Involvement::Authored, StateFilter::Any))
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            s.query(&q(Involvement::Reviewing, StateFilter::Active))
                .unwrap()[0]
                .id
                .number,
            2
        );
        s.set_starred(&cr(2, "", ChangeRequestState::Open).id, true)
            .unwrap();
        let mut sq = q(Involvement::All, StateFilter::Any);
        sq.starred_only = true;
        assert_eq!(s.query(&sq).unwrap().len(), 1);
        s.set_override(
            &cr(1, "", ChangeRequestState::Open).id.repo,
            &RepoOverride {
                alias: None,
                hidden: true,
            },
        )
        .unwrap();
        assert!(s
            .query(&q(Involvement::All, StateFilter::Any))
            .unwrap()
            .is_empty());
    }
}
