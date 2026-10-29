//! GitLab provider over REST v4.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::stream::{self, StreamExt};
use gv_auth::{Credential, CredentialSource, TokenOrigin};
use gv_core::*;
use serde_json::{json, Value};
use tokio::sync::{OnceCell, RwLock};

/// Max concurrent per-MR enrichment requests.
const CONCURRENCY: usize = 8;
const MAX_PAGES: usize = 10;

pub struct GitLabProvider {
    host: String,
    api: String,
    token: RwLock<String>,
    refreshable: bool,
    creds: CredentialSource,
    http: reqwest::Client,
    me: OnceCell<User>,
}

fn enc(path: &str) -> String {
    path.replace('/', "%2F")
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn time(v: &Value) -> DateTime<Utc> {
    v.as_str().and_then(|t| DateTime::parse_from_rfc3339(t).ok()).map(|t| t.with_timezone(&Utc)).unwrap_or_default()
}

fn user(v: &Value) -> User {
    User {
        username: v["username"].as_str().unwrap_or("unknown").to_string(),
        name: v["name"].as_str().map(str::to_string),
        avatar_url: v["avatar_url"].as_str().map(str::to_string),
    }
}

/// `https://host/group/proj/-/merge_requests/1` -> `group/proj`.
fn project_path_from_web_url(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let path = rest.split_once('/')?.1;
    Some(path.split_once("/-/")?.0.to_string())
}

fn note_to_comment(n: &Value) -> Comment {
    let pos = &n["position"];
    Comment {
        id: n["id"].as_u64().map(|i| i.to_string()).unwrap_or_default(),
        author: user(&n["author"]),
        body: s(&n["body"]),
        created_at: time(&n["created_at"]),
        position: pos["new_path"].as_str().or(pos["old_path"].as_str()).map(|p| InlinePosition {
            path: p.to_string(),
            line: pos["new_line"].as_u64().or(pos["old_line"].as_u64()).map(|l| l as u32),
        }),
    }
}

/// Map a GitLab MR (list or single shape) into a ChangeRequest, without reviewer states or checks.
fn parse_mr(host: &str, v: &Value) -> ChangeRequest {
    let path = project_path_from_web_url(v["web_url"].as_str().unwrap_or_default()).unwrap_or_default();
    let state = match v["state"].as_str() {
        Some("merged") => ChangeRequestState::Merged,
        Some("closed" | "locked") => ChangeRequestState::Closed,
        _ if v["draft"] == true || v["work_in_progress"] == true => ChangeRequestState::Draft,
        _ => ChangeRequestState::Open,
    };
    ChangeRequest {
        id: ChangeRequestId { repo: RepoRef::new(ProviderKind::GitLab, host, path), number: v["iid"].as_u64().unwrap_or(0) },
        title: s(&v["title"]),
        author: user(&v["author"]),
        state,
        source_branch: s(&v["source_branch"]),
        target_branch: s(&v["target_branch"]),
        reviewers: v["reviewers"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| Review { user: user(r), state: ReviewState::Pending })
            .collect(),
        checks: CheckSummary::default(),
        labels: v["labels"].as_array().into_iter().flatten().map(s).collect(),
        comments_count: v["user_notes_count"].as_u64().unwrap_or(0) as u32,
        additions: None,
        deletions: None,
        // "changes_count" is a string, may be "1000+".
        changed_files: v["changes_count"].as_str().and_then(|c| c.trim_end_matches('+').parse().ok()),
        updated_at: time(&v["updated_at"]),
        created_at: time(&v["created_at"]),
        web_url: s(&v["web_url"]),
        needs_you: false,
    }
}

/// Added and removed lines in one unified diff hunk text.
fn count_lines(diff: &str) -> (u32, u32) {
    diff.lines().fold((0, 0), |(a, d), l| {
        if l.starts_with("+++") || l.starts_with("---") {
            (a, d)
        } else if l.starts_with('+') {
            (a + 1, d)
        } else if l.starts_with('-') {
            (a, d + 1)
        } else {
            (a, d)
        }
    })
}

fn reviewer_state(s: &str) -> ReviewState {
    match s {
        "approved" => ReviewState::Approved,
        "requested_changes" => ReviewState::ChangesRequested,
        "reviewed" => ReviewState::Commented,
        _ => ReviewState::Pending,
    }
}

fn summarize_jobs(jobs: &[Value]) -> CheckSummary {
    let mut c = CheckSummary::default();
    // Retried jobs appear multiple times; the API returns the latest by default unless include_retried.
    for j in jobs {
        let st = j["status"].as_str().unwrap_or_default();
        if st == "skipped" || (st == "manual" && j["allow_failure"] == true) {
            continue;
        }
        c.total += 1;
        match st {
            "success" => c.passed += 1,
            "failed" if j["allow_failure"] == true => c.passed += 1,
            "failed" | "canceled" => c.failed += 1,
            _ => c.pending += 1,
        }
    }
    c
}

impl GitLabProvider {
    pub fn new(host: &str, cred: Credential) -> Self {
        let host = host.to_ascii_lowercase();
        Self {
            api: format!("https://{host}/api/v4"),
            host,
            refreshable: cred.origin != TokenOrigin::Env,
            token: RwLock::new(cred.token),
            creds: CredentialSource::new(),
            http: reqwest::Client::builder().user_agent("git-viewer").build().expect("http client"),
            me: OnceCell::new(),
        }
    }

    /// Send a request, refreshing the glab OAuth token once on 401. Returns (json, next page).
    async fn call(&self, method: reqwest::Method, path: &str, body: Option<&Value>) -> ProviderResult<(Value, Option<String>)> {
        let url = if path.starts_with("http") { path.to_string() } else { format!("{}{}", self.api, path) };
        for attempt in 0..2 {
            let token = self.token.read().await.clone();
            let mut req = self.http.request(method.clone(), &url).bearer_auth(token);
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = req.send().await.map_err(|e| ProviderError::Network(e.to_string()))?;
            let status = resp.status().as_u16();
            if status == 401 && attempt == 0 && self.refreshable {
                if let Ok(c) = self.creds.refresh_gitlab(&self.host).await {
                    *self.token.write().await = c.token;
                    continue;
                }
            }
            let next = resp.headers().get("x-next-page").and_then(|h| h.to_str().ok()).filter(|s| !s.is_empty()).map(str::to_string);
            let text = resp.text().await.map_err(|e| ProviderError::Network(e.to_string()))?;
            return match status {
                200..=299 => Ok((serde_json::from_str(&text).map_err(|e| ProviderError::Decode(e.to_string()))?, next)),
                401 => Err(ProviderError::Unauthenticated(self.host.clone())),
                404 => Err(ProviderError::NotFound(url)),
                _ => Err(ProviderError::Http { status, body: text }),
            };
        }
        Err(ProviderError::Unauthenticated(self.host.clone()))
    }

    async fn get(&self, path: &str) -> ProviderResult<Value> {
        Ok(self.call(reqwest::Method::GET, path, None).await?.0)
    }

    async fn get_all(&self, path: &str) -> ProviderResult<Vec<Value>> {
        let sep = if path.contains('?') { '&' } else { '?' };
        let mut out = vec![];
        let mut page = "1".to_string();
        for _ in 0..MAX_PAGES {
            let (v, next) = self.call(reqwest::Method::GET, &format!("{path}{sep}per_page=100&page={page}"), None).await?;
            out.extend(v.as_array().cloned().unwrap_or_default());
            match next {
                Some(n) => page = n,
                None => break,
            }
        }
        Ok(out)
    }

    async fn me(&self) -> ProviderResult<&User> {
        self.me
            .get_or_try_init(|| async { Ok(user(&self.get("/user").await?)) })
            .await
    }

    fn mr_path(id: &ChangeRequestId) -> String {
        format!("/projects/{}/merge_requests/{}", enc(&id.repo.path), id.number)
    }

    /// Fill in reviewer states, check summary and change count with extra calls.
    async fn enrich(&self, mut cr: ChangeRequest, me: &str) -> ChangeRequest {
        let base = Self::mr_path(&cr.id);
        if let Ok(rs) = self.get(&format!("{base}/reviewers")).await {
            cr.reviewers = rs
                .as_array()
                .into_iter()
                .flatten()
                .map(|r| Review { user: user(&r["user"]), state: reviewer_state(r["state"].as_str().unwrap_or_default()) })
                .collect();
        }
        if let Ok(full) = self.get(&base).await {
            cr.changed_files = full["changes_count"].as_str().and_then(|c| c.trim_end_matches('+').parse().ok());
            if let Some(pid) = full["head_pipeline"]["id"].as_u64() {
                let proj = full["head_pipeline"]["project_id"].as_u64().map(|p| p.to_string()).unwrap_or_else(|| enc(&cr.id.repo.path));
                if let Ok(jobs) = self.get_all(&format!("/projects/{proj}/pipelines/{pid}/jobs")).await {
                    cr.checks = summarize_jobs(&jobs);
                }
            }
        }
        cr.needs_you = compute_needs_you(&cr, me);
        cr
    }

    /// Added and removed line counts from the MR diffs (detail only; the list has none).
    /// Falls back to the older `changes` endpoint on GitLab before 15.7.
    async fn diff_stats(&self, base: &str) -> Option<(u32, u32)> {
        let diffs = match self.get_all(&format!("{base}/diffs")).await {
            Ok(d) => d,
            Err(_) => self.get(&format!("{base}/changes")).await.ok()?["changes"].as_array()?.clone(),
        };
        Some(diffs.iter().filter_map(|d| d["diff"].as_str()).map(count_lines).fold((0, 0), |(a, d), (x, y)| (a + x, d + y)))
    }

    async fn enrich_all(&self, items: Vec<ChangeRequest>, me: &str) -> Vec<ChangeRequest> {
        stream::iter(items).map(|cr| self.enrich(cr, me)).buffer_unordered(CONCURRENCY).collect().await
    }
}

#[async_trait]
impl Provider for GitLabProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::GitLab
    }

    fn host(&self) -> &str {
        &self.host
    }

    async fn whoami(&self) -> ProviderResult<User> {
        Ok(self.me().await?.clone())
    }

    async fn list_change_requests(&self, scope: &Scope, filter: &Filter) -> ProviderResult<Vec<ChangeRequest>> {
        let me = self.me().await?.username.clone();
        let state = match filter.state {
            StateFilter::Active => "opened",
            StateFilter::Merged => "merged",
            StateFilter::Closed => "closed",
            StateFilter::Any => "all",
        };
        let base = match scope {
            Scope::Repo(r) => format!("/projects/{}/merge_requests", enc(&r.path)),
            Scope::Group { owner, .. } => format!("/groups/{}/merge_requests", enc(owner)),
            Scope::InvolvingMe { .. } => "/merge_requests".to_string(),
        };
        let common = format!("{base}?state={state}&scope=all&order_by=updated_at");
        let mut queries = vec![];
        if matches!(filter.involvement, Involvement::Authored | Involvement::All) {
            queries.push(format!("{common}&author_username={me}"));
        }
        if matches!(filter.involvement, Involvement::Reviewing | Involvement::All) {
            queries.push(format!("{common}&reviewer_username={me}"));
        }
        if filter.involvement == Involvement::All {
            if matches!(scope, Scope::InvolvingMe { .. }) {
                queries.push(format!("{common}&assignee_username={me}"));
            } else {
                // Repo/group scope with "All" means every MR there.
                queries = vec![common];
            }
        }
        let mut raw: Vec<ChangeRequest> = vec![];
        for q in queries {
            for v in self.get_all(&q).await? {
                let cr = parse_mr(&self.host, &v);
                if !raw.iter().any(|x| x.id == cr.id) {
                    raw.push(cr);
                }
            }
        }
        let mut out = self.enrich_all(raw, &me).await;
        out.sort_by_key(|c| std::cmp::Reverse(c.updated_at));
        Ok(out)
    }

    async fn get_change_request(&self, id: &ChangeRequestId) -> ProviderResult<ChangeRequestDetail> {
        let me = self.me().await?.username.clone();
        let base = Self::mr_path(id);
        let v = self.get(&base).await?;
        let mut summary = self.enrich(parse_mr(&self.host, &v), &me).await;
        if let Some((a, d)) = self.diff_stats(&base).await {
            summary.additions = Some(a);
            summary.deletions = Some(d);
        }
        let discussions = self.get_all(&format!("{base}/discussions")).await?;
        let threads = discussions
            .iter()
            .filter_map(|d| {
                let notes: Vec<&Value> = d["notes"].as_array()?.iter().filter(|n| n["system"] != true).collect();
                if notes.is_empty() {
                    return None;
                }
                let resolvable = notes.iter().any(|n| n["resolvable"] == true);
                Some(Thread {
                    id: s(&d["id"]),
                    resolved: resolvable.then(|| notes.iter().all(|n| n["resolved"] == true)),
                    comments: notes.into_iter().map(note_to_comment).collect(),
                })
            })
            .collect();
        Ok(ChangeRequestDetail {
            summary,
            description: s(&v["description"]),
            threads,
            head_sha: v["sha"].as_str().map(str::to_string),
        })
    }

    async fn list_repos(&self, owner: &str) -> ProviderResult<Vec<RepoRef>> {
        let path = format!("/groups/{}/projects?include_subgroups=true&archived=false&simple=true", enc(owner));
        let items = match self.get_all(&path).await {
            Ok(v) => v,
            Err(ProviderError::NotFound(_)) => self.get_all(&format!("/users/{}/projects?archived=false&simple=true", enc(owner))).await?,
            Err(e) => return Err(e),
        };
        Ok(items.iter().map(|p| RepoRef::new(ProviderKind::GitLab, &self.host, s(&p["path_with_namespace"]))).collect())
    }

    async fn add_comment(&self, id: &ChangeRequestId, body: &str) -> ProviderResult<Comment> {
        let (v, _) = self.call(reqwest::Method::POST, &format!("{}/notes", Self::mr_path(id)), Some(&json!({"body": body}))).await?;
        Ok(note_to_comment(&v))
    }

    async fn reply_to_thread(&self, id: &ChangeRequestId, thread_id: &str, body: &str) -> ProviderResult<Comment> {
        let path = format!("{}/discussions/{thread_id}/notes", Self::mr_path(id));
        let (v, _) = self.call(reqwest::Method::POST, &path, Some(&json!({"body": body}))).await?;
        Ok(note_to_comment(&v))
    }

    async fn edit_comment(&self, id: &ChangeRequestId, comment_id: &str, body: &str) -> ProviderResult<Comment> {
        let path = format!("{}/notes/{comment_id}", Self::mr_path(id));
        let (v, _) = self.call(reqwest::Method::PUT, &path, Some(&json!({"body": body}))).await?;
        Ok(note_to_comment(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mr() {
        let v = json!({"iid": 7, "title": "x", "state": "opened", "draft": true,
            "web_url": "https://gitlab.com/g/sub/p/-/merge_requests/7", "author": {"username": "a"},
            "reviewers": [{"username": "me"}], "labels": ["l"], "user_notes_count": 2,
            "created_at": "2026-01-01T00:00:00.000Z", "updated_at": "2026-01-02T00:00:00.000Z"});
        let cr = parse_mr("gitlab.com", &v);
        assert_eq!(cr.id.repo.path, "g/sub/p");
        assert_eq!(cr.state, ChangeRequestState::Draft);
        assert_eq!(cr.reviewers[0].state, ReviewState::Pending);
    }

    #[test]
    fn counts_diff_lines() {
        assert_eq!(count_lines("@@ -1,2 +1,2 @@\n-old\n+new\n+more\n ctx"), (2, 1));
    }

    #[test]
    fn job_summary() {
        let jobs = vec![
            json!({"status": "success"}),
            json!({"status": "failed", "allow_failure": true}),
            json!({"status": "failed"}),
            json!({"status": "running"}),
            json!({"status": "skipped"}),
        ];
        assert_eq!(summarize_jobs(&jobs), CheckSummary { total: 4, passed: 2, failed: 1, pending: 1 });
    }
}
