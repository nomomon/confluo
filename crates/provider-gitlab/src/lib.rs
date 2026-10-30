//! GitLab provider over REST v4.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::stream::{self, StreamExt};
use gv_auth::{Credential, CredentialSource, TokenOrigin};
use gv_core::*;
use serde_json::{json, Value};
use tokio::sync::{OnceCell, RwLock};

mod checks;
mod edit;
mod extras;
mod inline;
mod review;

/// Concurrent per-MR enrichments; the host governor caps requests in flight anyway.
const CONCURRENCY: usize = 4;
const MAX_PAGES: usize = 10;

pub struct GitLabProvider {
    host: String,
    api: String,
    token: RwLock<String>,
    refreshable: bool,
    creds: CredentialSource,
    http: reqwest::Client,
    me: OnceCell<User>,
    /// Instance has the GraphQL `mergeRequestRequestChanges` mutation (probed once).
    request_changes: OnceCell<bool>,
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
        bot: v["bot"] == true,
        id: v["id"].as_u64().map(|i| i.to_string()),
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
        reactions: vec![],
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

/// One entry of `/merge_requests/:iid/diffs` (or `/changes`).
fn changed_file(v: &Value) -> ChangedFile {
    let status = if v["new_file"] == true {
        FileStatus::Added
    } else if v["deleted_file"] == true {
        FileStatus::Deleted
    } else if v["renamed_file"] == true {
        FileStatus::Renamed
    } else {
        FileStatus::Modified
    };
    let diff = v["diff"].as_str().unwrap_or_default();
    let mut f = ChangedFile::from_patch(&s(&v["old_path"]), &s(&v["new_path"]), status, diff);
    let flagged = v["too_large"] == true || v["collapsed"] == true;
    if diff.is_empty() && status != FileStatus::Renamed {
        f.too_large = flagged;
        f.binary = !flagged;
    } else if flagged && f.hunks.is_empty() {
        f.too_large = true;
    }
    f
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
            request_changes: OnceCell::new(),
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
            let resp = gv_http::send(req).await?;
            let status = resp.status;
            if status == 401 && attempt == 0 && self.refreshable {
                if let Ok(c) = self.creds.refresh_gitlab(&self.host).await {
                    *self.token.write().await = c.token;
                    continue;
                }
            }
            let next = resp.header("x-next-page").filter(|s| !s.is_empty()).map(str::to_string);
            let text = resp.text();
            return match status {
                // DELETE answers 204 with no body.
                200..=299 if text.trim().is_empty() => Ok((Value::Null, next)),
                200..=299 => Ok((serde_json::from_str(&text).map_err(|e| ProviderError::Decode(e.to_string()))?, next)),
                401 => Err(ProviderError::Unauthenticated(self.host.clone())),
                404 => Err(ProviderError::NotFound(url)),
                _ => Err(ProviderError::Http { status, body: text }),
            };
        }
        Err(ProviderError::Unauthenticated(self.host.clone()))
    }

    async fn graphql(&self, body: &Value) -> ProviderResult<Value> {
        Ok(self.call(reqwest::Method::POST, &format!("https://{}/api/graphql", self.host), Some(body)).await?.0)
    }

    /// Whether "Request changes" is available on this instance (one read-only schema probe, cached).
    async fn can_request_changes(&self) -> bool {
        *self
            .request_changes
            .get_or_init(|| async { self.graphql(&json!({ "query": review::REQUEST_CHANGES_PROBE })).await.map(|v| review::supports_request_changes(&v)).unwrap_or(false) })
            .await
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

    /// Fill in reviewer states, check summary and change count with extra calls (up to three).
    /// `full` is the single-MR payload when the caller already has it.
    async fn enrich_one(&self, cr: ChangeRequest, me: &str, full: Option<&Value>) -> ProviderResult<ChangeRequest> {
        Ok(self.enrich_jobs(cr, me, full).await?.0)
    }

    /// [`Self::enrich_one`] that also returns the head pipeline's jobs and `project/pipeline` id.
    async fn enrich_jobs(&self, mut cr: ChangeRequest, me: &str, full: Option<&Value>) -> ProviderResult<(ChangeRequest, Vec<Value>, Option<String>)> {
        let mut jobs = vec![];
        let mut pipeline = None;
        let base = Self::mr_path(&cr.id);
        let rs = self.get(&format!("{base}/reviewers")).await?;
        cr.reviewers = rs
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| Review { user: user(&r["user"]), state: reviewer_state(r["state"].as_str().unwrap_or_default()) })
            .collect();
        let fetched = match full {
            Some(_) => None,
            None => Some(self.get(&base).await?),
        };
        if let Some(full) = full.or(fetched.as_ref()) {
            cr.changed_files = full["changes_count"].as_str().and_then(|c| c.trim_end_matches('+').parse().ok());
            if let Some(pid) = full["head_pipeline"]["id"].as_u64() {
                let proj = full["head_pipeline"]["project_id"].as_u64().map(|p| p.to_string()).unwrap_or_else(|| enc(&cr.id.repo.path));
                jobs = self.get_all(&format!("/projects/{proj}/pipelines/{pid}/jobs")).await?;
                cr.checks = summarize_jobs(&jobs);
                pipeline = Some(format!("{proj}/{pid}"));
            }
        }
        cr.needs_you = compute_needs_you(&cr, me);
        Ok((cr, jobs, pipeline))
    }

    /// Raw MR diff entries. Falls back to the older `changes` endpoint on GitLab before 15.7.
    async fn raw_diffs(&self, base: &str) -> ProviderResult<Vec<Value>> {
        match self.get_all(&format!("{base}/diffs")).await {
            Ok(d) => Ok(d),
            Err(_) => Ok(self.get(&format!("{base}/changes")).await?["changes"].as_array().cloned().unwrap_or_default()),
        }
    }

    /// Added and removed line counts from the MR diffs (detail only; the list has none).
    async fn diff_stats(&self, base: &str) -> Option<(u32, u32)> {
        let diffs = self.raw_diffs(base).await.ok()?;
        Some(diffs.iter().filter_map(|d| d["diff"].as_str()).map(count_lines).fold((0, 0), |(a, d), (x, y)| (a + x, d + y)))
    }

    /// GET returning raw text; None on 404.
    async fn get_text(&self, path: &str) -> ProviderResult<Option<String>> {
        let token = self.token.read().await.clone();
        let resp = gv_http::send(self.http.get(format!("{}{}", self.api, path)).bearer_auth(token)).await?;
        let (status, text) = (resp.status, resp.text());
        match status {
            200..=299 => Ok(Some(text)),
            404 => Ok(None),
            401 => Err(ProviderError::Unauthenticated(self.host.clone())),
            _ => Err(ProviderError::Http { status, body: text }),
        }
    }

    async fn enrich_all(&self, items: Vec<ChangeRequest>, me: &str) -> Vec<ProviderResult<ChangeRequest>> {
        stream::iter(items).map(|cr| self.enrich_one(cr, me, None)).buffered(CONCURRENCY).collect().await
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

    fn lists_need_enrich(&self) -> bool {
        true
    }

    async fn enrich(&self, items: Vec<ChangeRequest>) -> Vec<ProviderResult<ChangeRequest>> {
        let me = match self.me().await {
            Ok(u) => u.username.clone(),
            Err(e) => {
                let msg = e.to_string();
                return items.into_iter().map(|_| Err(ProviderError::Network(msg.clone()))).collect();
            }
        };
        self.enrich_all(items, &me).await
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
        // Reviewer states and checks come from `enrich`, which the engine calls only for MRs
        // that changed since the cache.
        let mut out = raw;
        for cr in &mut out {
            cr.needs_you = compute_needs_you(cr, &me);
        }
        out.sort_by_key(|c| std::cmp::Reverse(c.updated_at));
        Ok(out)
    }

    async fn get_change_request(&self, id: &ChangeRequestId) -> ProviderResult<ChangeRequestDetail> {
        let me = self.me().await?.username.clone();
        let base = Self::mr_path(id);
        let v = self.get(&base).await?;
        let (mut summary, jobs, pipeline) = self.enrich_jobs(parse_mr(&self.host, &v), &me, Some(&v)).await?;
        let edit = edit::edit_info(&v, &summary, &me);
        let checks = checks::checks_info(&jobs, pipeline, v["user"]["can_merge"] == true);
        if let Some((a, d)) = self.diff_stats(&base).await {
            summary.additions = Some(a);
            summary.deletions = Some(d);
        }
        let discussions = self.get_all(&format!("{base}/discussions")).await?;
        // One request each; award emoji per note would be one per note, so notes carry none.
        let awards = self.get(&format!("{base}/award_emoji")).await.ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let commits = self.get(&format!("{base}/commits?per_page=100")).await.ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let review = match self.get(&format!("{base}/approvals")).await {
            Ok(a) => review::review_info(&summary, &me, &a),
            Err(_) => ReviewInfo { caps: gitlab_capabilities(&summary, &me, false, !summary.author.username.eq_ignore_ascii_case(&me)), my_state: my_review_state(&summary, &me), approvals: None },
        };
        let mut review = review;
        review.caps.request_changes = summary.state.is_active() && !summary.author.username.eq_ignore_ascii_case(&me) && self.can_request_changes().await;
        let mut events = vec![TimelineEvent { kind: TimelineKind::Opened, actor: summary.author.clone(), at: summary.created_at }];
        events.extend(discussions.iter().flat_map(|d| d["notes"].as_array().into_iter().flatten()).filter(|n| n["system"] == true).filter_map(review::system_event));
        events.sort_by_key(|e| e.at);
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
                    resolved: resolvable.then(|| notes.iter().filter(|n| n["resolvable"] == true).all(|n| n["resolved"] == true)),
                    position: inline::parse_position(&notes[0]["position"]),
                    comments: notes.into_iter().map(note_to_comment).collect(),
                    outdated: false,
                })
            })
            .collect();
        Ok(ChangeRequestDetail {
            review,
            events,
            edit,
            checks,
            summary,
            description: s(&v["description"]),
            threads,
            head_sha: v["sha"].as_str().map(str::to_string),
            reactions: extras::awards(&awards, &me),
            commits: extras::commits(&commits),
            my_review_sha: None,
            viewed: None,
            node_id: None,
        })
    }

    async fn react(&self, id: &ChangeRequestId, subject: &str, kind: ReactionKind, add: bool) -> ProviderResult<()> {
        self.do_react(id, subject, kind, add).await
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

    async fn add_line_comment(&self, id: &ChangeRequestId, position: &DiffPosition, body: &str) -> ProviderResult<Comment> {
        let base = Self::mr_path(id);
        let mr = self.get(&base).await?;
        let r = &mr["diff_refs"];
        let refs = (
            r["base_sha"].as_str().unwrap_or_default(),
            r["start_sha"].as_str().unwrap_or_default(),
            r["head_sha"].as_str().unwrap_or_default(),
        );
        let payload = inline::discussion_payload(position, refs, body);
        let (v, _) = self.call(reqwest::Method::POST, &format!("{base}/discussions"), Some(&payload)).await?;
        Ok(note_to_comment(&v["notes"][0]))
    }

    async fn resolve_thread(&self, id: &ChangeRequestId, thread_id: &str, resolved: bool) -> ProviderResult<()> {
        let path = format!("{}/discussions/{thread_id}?resolved={resolved}", Self::mr_path(id));
        self.call(reqwest::Method::PUT, &path, None).await?;
        Ok(())
    }

    async fn list_changed_files(&self, id: &ChangeRequestId) -> ProviderResult<Vec<ChangedFile>> {
        Ok(self.raw_diffs(&Self::mr_path(id)).await?.iter().map(changed_file).collect())
    }

    async fn get_file_content(&self, id: &ChangeRequestId, path: &str, rev: &str) -> ProviderResult<Option<String>> {
        let p = path.replace('%', "%25").replace('/', "%2F").replace(' ', "%20").replace('#', "%23").replace('?', "%3F").replace('&', "%26");
        self.get_text(&format!("/projects/{}/repository/files/{p}/raw?ref={rev}", enc(&id.repo.path))).await
    }

    async fn submit_review(&self, id: &ChangeRequestId, action: ReviewAction, body: Option<&str>) -> ProviderResult<()> {
        if action == ReviewAction::RequestChanges {
            if !self.can_request_changes().await {
                return Err(ProviderError::Unsupported("Request changes on this GitLab version".into()));
            }
            let resp = self.graphql(&review::request_changes_payload(&id.repo.path, id.number)).await?;
            if let Some(e) = review::graphql_error(&resp, "mergeRequestRequestChanges") {
                return Err(ProviderError::Http { status: 422, body: e });
            }
            if let Some(b) = body {
                self.call(reqwest::Method::POST, &format!("{}/notes", Self::mr_path(id)), Some(&json!({"body": b}))).await?;
            }
            return Ok(());
        }
        let (method, suffix) = review::review_request(action).ok_or_else(|| ProviderError::Unsupported(format!("{} on GitLab", action.label())))?;
        let base = Self::mr_path(id);
        if action == ReviewAction::Comment {
            let Some(b) = body else { return Ok(()) };
            self.call(method, &format!("{base}{suffix}"), Some(&json!({"body": b}))).await?;
            return Ok(());
        }
        self.call(method, &format!("{base}{suffix}"), None).await?;
        // A summary with an approval goes in as a note.
        if let Some(b) = body {
            self.call(reqwest::Method::POST, &format!("{base}/notes"), Some(&json!({"body": b}))).await?;
        }
        Ok(())
    }

    async fn update_change_request(&self, id: &ChangeRequestId, title: Option<&str>, body: Option<&str>) -> ProviderResult<()> {
        self.do_update(id, title, body).await
    }

    async fn set_people(&self, id: &ChangeRequestId, role: PeopleRole, _current: &[User], desired: &[User]) -> ProviderResult<()> {
        self.do_set_people(id, role, desired).await
    }

    async fn search_users(&self, id: &ChangeRequestId, query: &str) -> ProviderResult<Vec<User>> {
        self.do_search_users(id, query).await
    }

    async fn set_draft(&self, id: &ChangeRequestId, draft: bool, title: &str, _node_id: Option<&str>) -> ProviderResult<()> {
        self.do_set_draft(id, draft, title).await
    }

    async fn set_open(&self, id: &ChangeRequestId, open: bool) -> ProviderResult<()> {
        self.do_set_open(id, open).await
    }

    async fn merge(&self, id: &ChangeRequestId, opts: &MergeOptions) -> ProviderResult<()> {
        self.do_merge(id, opts).await
    }

    async fn check_log(&self, id: &ChangeRequestId, run: &CheckRun) -> ProviderResult<Vec<String>> {
        self.do_check_log(id, run).await
    }

    async fn rerun_failed(&self, _id: &ChangeRequestId, target: &str) -> ProviderResult<()> {
        self.do_rerun(target).await
    }

    async fn delete_comment(&self, id: &ChangeRequestId, comment_id: &str) -> ProviderResult<()> {
        self.call(reqwest::Method::DELETE, &delete_note_path(&Self::mr_path(id), comment_id), None).await.map(|_| ())
    }

    async fn edit_comment(&self, id: &ChangeRequestId, comment_id: &str, body: &str) -> ProviderResult<Comment> {
        let path = format!("{}/notes/{comment_id}", Self::mr_path(id));
        let (v, _) = self.call(reqwest::Method::PUT, &path, Some(&json!({"body": body}))).await?;
        Ok(note_to_comment(&v))
    }
}

/// `DELETE /projects/:id/merge_requests/:iid/notes/:note_id`.
fn delete_note_path(mr_path: &str, note_id: &str) -> String {
    format!("{mr_path}/notes/{note_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_note_path_targets_the_note() {
        assert_eq!(delete_note_path("/projects/g%2Fp/merge_requests/7", "42"), "/projects/g%2Fp/merge_requests/7/notes/42");
    }

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
    fn parses_changed_file() {
        let f = changed_file(&json!({"old_path": "a", "new_path": "b", "renamed_file": true, "diff": "@@ -1 +1 @@\n-x\n+y\n"}));
        assert_eq!((f.status, f.additions, f.deletions), (FileStatus::Renamed, 1, 1));
        let big = changed_file(&json!({"old_path": "a", "new_path": "a", "diff": "", "too_large": true}));
        assert!(big.too_large && !big.binary);
        let bin = changed_file(&json!({"old_path": "i.png", "new_path": "i.png", "new_file": true, "diff": ""}));
        assert!(bin.binary);
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
