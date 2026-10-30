//! Azure DevOps (Azure Repos pull requests) over REST api-version 7.1.
//!
//! Repos are `org/project/repo` on host `dev.azure.com`. Auth is a Bearer token from
//! `az account get-access-token` or a PAT (env or keychain, per organization, sent as Basic).

use async_trait::async_trait;
use futures_util::stream::{self, StreamExt};
use gv_auth::{Credential, CredentialSource};
use gv_core::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::{OnceCell, RwLock};

mod auth;
mod checks;
mod commit_diff;
mod diff;
mod edit;
mod parse;

use parse::*;
pub use parse::{vote_action, vote_state};

const API: &str = "api-version=7.1";
const API_PREVIEW: &str = "api-version=7.1-preview.1";
const CONCURRENCY: usize = 4;
const PAGE: usize = 100;
const MAX_PAGES: usize = 5;

pub struct AzureProvider {
    host: String,
    /// The host-wide credential (az token, env PAT or host keychain PAT).
    cred: RwLock<Credential>,
    /// Keychain PATs per organization (lowercase), used for URLs under that org.
    org_creds: HashMap<String, Credential>,
    creds: CredentialSource,
    http: reqwest::Client,
    me: OnceCell<User>,
    /// Organizations to search for "involving me" (from the user's projects).
    orgs: Mutex<Vec<String>>,
    /// `org/project` name -> project GUID (policy evaluations need it).
    project_ids: Mutex<HashMap<String, String>>,
    /// PR key -> last merge source commit, seen in lists (enrichment marker).
    heads: Mutex<HashMap<String, String>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl AzureProvider {
    pub fn new(host: &str, cred: Credential) -> Self {
        Self {
            host: host.to_ascii_lowercase(),
            cred: RwLock::new(cred),
            org_creds: HashMap::new(),
            creds: CredentialSource::new(),
            http: reqwest::Client::builder()
                .user_agent("git-viewer")
                .build()
                .expect("http client"),
            me: OnceCell::new(),
            orgs: Mutex::new(vec![]),
            project_ids: Mutex::new(HashMap::new()),
            heads: Mutex::new(HashMap::new()),
        }
    }

    /// Send a request; on 401 ask `az` for a fresh token once. Returns the raw response.
    async fn send(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<&Value>,
    ) -> ProviderResult<gv_http::Resp> {
        for attempt in 0..2 {
            let (cred, org_scoped) = self.cred_for(url).await;
            let (name, value) = cred.header();
            let mut req = self
                .http
                .request(method.clone(), url)
                .header(name, value)
                .header("Accept", "application/json");
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = gv_http::send(req).await?;
            if resp.status == 401 && attempt == 0 && !org_scoped && cred.refreshable() {
                if let Ok(c) = self.creds.get(ProviderKind::Azure, &self.host).await {
                    *self.cred.write().await = c;
                    continue;
                }
            }
            return Ok(resp);
        }
        Err(ProviderError::Unauthenticated(self.host.clone()))
    }

    async fn call(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<&Value>,
    ) -> ProviderResult<Value> {
        let resp = self.send(method, url, body).await?;
        let status = resp.status;
        // An invalid PAT gets a 203 with the sign-in page instead of a 401.
        let html = resp
            .header("content-type")
            .is_some_and(|c| c.contains("text/html"));
        let text = resp.text();
        match status {
            203 => Err(ProviderError::Unauthenticated(self.host.clone())),
            200..=299 if html => Err(ProviderError::Unauthenticated(self.host.clone())),
            204 => Ok(Value::Null),
            200..=299 if text.trim().is_empty() => Ok(Value::Null),
            200..=299 => {
                serde_json::from_str(&text).map_err(|e| ProviderError::Decode(e.to_string()))
            }
            401 => Err(ProviderError::Unauthenticated(self.host.clone())),
            404 => Err(ProviderError::NotFound(url.to_string())),
            _ => Err(ProviderError::Http {
                status,
                body: text.chars().take(500).collect(),
            }),
        }
    }

    async fn get(&self, url: &str) -> ProviderResult<Value> {
        self.call(reqwest::Method::GET, url, None).await
    }

    /// GET raw bytes; None on 404.
    async fn get_bytes(&self, url: &str) -> ProviderResult<Option<Vec<u8>>> {
        let resp = self.send(reqwest::Method::GET, url, None).await?;
        match resp.status {
            200..=299 if resp.status != 203 => Ok(Some(resp.body)),
            203 | 401 => Err(ProviderError::Unauthenticated(self.host.clone())),
            404 => Ok(None),
            s => Err(ProviderError::Http {
                status: s,
                body: resp.text().chars().take(500).collect(),
            }),
        }
    }

    /// `value` arrays over `$top`/`$skip` pages.
    async fn get_paged(&self, url: &str) -> ProviderResult<Vec<Value>> {
        let mut out = vec![];
        for page in 0..MAX_PAGES {
            let v = self
                .get(&format!("{url}&$top={PAGE}&$skip={}", page * PAGE))
                .await?;
            let items = v["value"].as_array().cloned().unwrap_or_default();
            let n = items.len();
            out.extend(items);
            if n < PAGE {
                break;
            }
        }
        Ok(out)
    }

    fn orgs(&self) -> Vec<String> {
        lock(&self.orgs).clone()
    }

    async fn me(&self) -> ProviderResult<&User> {
        self.me
            .get_or_try_init(|| async {
                // connectionData names the identity the org knows; the profile is the fallback without an org.
                if let Some(org) = self.orgs().first() {
                    if let Ok(v) = self
                        .get(&format!(
                            "https://{}/{}/_apis/connectionData",
                            self.host,
                            seg(org)
                        ))
                        .await
                    {
                        if let Some(u) = connection_user(&v) {
                            return Ok(u);
                        }
                    }
                }
                let v = self
                    .get(&format!(
                        "https://app.vssps.visualstudio.com/_apis/profile/profiles/me?{API}"
                    ))
                    .await?;
                profile_user(&v).ok_or_else(|| ProviderError::Decode("profile has no id".into()))
            })
            .await
    }

    fn remember(&self, items: &[(ChangeRequest, Value)]) {
        let mut ids = lock(&self.project_ids);
        let mut heads = lock(&self.heads);
        for (cr, v) in items {
            if let (Some(pid), Some((org, project, _))) = (
                v["repository"]["project"]["id"].as_str(),
                split_repo(&cr.id.repo.path),
            ) {
                ids.insert(format!("{org}/{project}"), pid.to_string());
            }
            if let Some(h) = v["lastMergeSourceCommit"]["commitId"].as_str() {
                heads.insert(cr.id.key(), h.to_string());
            }
        }
    }

    /// Project GUID for `org/project` (from lists, else one request).
    async fn project_id(&self, org: &str, project: &str) -> ProviderResult<String> {
        let key = format!("{org}/{project}");
        if let Some(id) = lock(&self.project_ids).get(&key) {
            return Ok(id.clone());
        }
        let v = self
            .get(&format!(
                "https://{}/{}/_apis/projects/{}?{API}",
                self.host,
                seg(org),
                seg(project)
            ))
            .await?;
        let id = v["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Decode("project without id".into()))?
            .to_string();
        lock(&self.project_ids).insert(key, id.clone());
        Ok(id)
    }

    fn pr_url(&self, id: &ChangeRequestId) -> ProviderResult<String> {
        let (org, project, repo) = split_repo(&id.repo.path)
            .ok_or_else(|| ProviderError::NotFound(id.repo.path.clone()))?;
        Ok(format!(
            "https://{}/{}/{}/_apis/git/repositories/{}/pullRequests/{}",
            self.host,
            seg(org),
            seg(project),
            seg(repo),
            id.number
        ))
    }

    fn repo_url(&self, id: &ChangeRequestId) -> ProviderResult<String> {
        let (org, project, repo) = split_repo(&id.repo.path)
            .ok_or_else(|| ProviderError::NotFound(id.repo.path.clone()))?;
        Ok(format!(
            "https://{}/{}/{}/_apis/git/repositories/{}",
            self.host,
            seg(org),
            seg(project),
            seg(repo)
        ))
    }

    /// Policy evaluations of a PR (build validation, reviewers, comments).
    async fn policy_evaluations(&self, id: &ChangeRequestId) -> ProviderResult<Vec<Value>> {
        let (org, project, _) = split_repo(&id.repo.path)
            .ok_or_else(|| ProviderError::NotFound(id.repo.path.clone()))?;
        let pid = self.project_id(org, project).await?;
        let artifact = format!("vstfs:///CodeReview/CodeReviewId/{pid}/{}", id.number);
        let url = format!(
            "https://{}/{}/{}/_apis/policy/evaluations?artifactId={}&{API_PREVIEW}",
            self.host,
            seg(org),
            seg(project),
            query(&artifact)
        );
        Ok(self.get(&url).await?["value"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    async fn enrich_one(&self, mut cr: ChangeRequest, me: &User) -> ProviderResult<ChangeRequest> {
        let evals = self.policy_evaluations(&cr.id).await?;
        let (org, project, _) = split_repo(&cr.id.repo.path).unwrap_or_default();
        cr.checks = checks::summary(&checks::policy_runs(&evals, org, project));
        cr.needs_you = compute_needs_you(&cr, &me.username);
        Ok(cr)
    }

    /// One thread's first comment as a Comment (for create responses).
    fn first_comment(thread: &Value) -> ProviderResult<Comment> {
        let tid = thread["id"].as_u64().unwrap_or(0);
        thread["comments"]
            .as_array()
            .and_then(|c| c.first())
            .map(|c| comment(tid, c))
            .ok_or_else(|| ProviderError::Decode("thread without comments".into()))
    }
}

#[async_trait]
impl Provider for AzureProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Azure
    }

    fn host(&self) -> &str {
        &self.host
    }

    fn set_namespaces(&self, namespaces: Vec<String>) {
        *lock(&self.orgs) = namespaces;
    }

    async fn whoami(&self) -> ProviderResult<User> {
        Ok(self.me().await?.clone())
    }

    fn lists_need_enrich(&self) -> bool {
        true
    }

    fn enrich_fills_reviewers(&self) -> bool {
        false
    }

    fn enrich_marker(&self, cr: &ChangeRequest) -> String {
        let head = lock(&self.heads)
            .get(&cr.id.key())
            .cloned()
            .unwrap_or_default();
        format!("{}|{head}", cr.updated_at.to_rfc3339())
    }

    async fn enrich(&self, items: Vec<ChangeRequest>) -> Vec<ProviderResult<ChangeRequest>> {
        let me = match self.me().await {
            Ok(u) => u.clone(),
            Err(e) => {
                let msg = e.to_string();
                return items
                    .into_iter()
                    .map(|_| Err(ProviderError::Network(msg.clone())))
                    .collect();
            }
        };
        stream::iter(items)
            .map(|cr| self.enrich_one(cr, &me))
            .buffered(CONCURRENCY)
            .collect()
            .await
    }

    async fn list_change_requests(
        &self,
        scope: &Scope,
        filter: &Filter,
    ) -> ProviderResult<Vec<ChangeRequest>> {
        let me = self.me().await?.clone();
        let my_id = me.id.clone().unwrap_or_default();
        let status = match filter.state {
            StateFilter::Active => "active",
            StateFilter::Merged => "completed",
            StateFilter::Closed => "abandoned",
            StateFilter::Any => "all",
        };
        // (org, list url without query)
        let bases: Vec<(String, String)> = match scope {
            Scope::Repo(r) => {
                let (org, project, repo) =
                    split_repo(&r.path).ok_or_else(|| ProviderError::NotFound(r.path.clone()))?;
                vec![(
                    org.to_string(),
                    format!(
                        "https://{}/{}/{}/_apis/git/repositories/{}/pullrequests",
                        self.host,
                        seg(org),
                        seg(project),
                        seg(repo)
                    ),
                )]
            }
            Scope::Group { owner, .. } => match owner.split_once('/') {
                Some((org, project)) => vec![(
                    org.to_string(),
                    format!(
                        "https://{}/{}/{}/_apis/git/pullrequests",
                        self.host,
                        seg(org),
                        seg(project)
                    ),
                )],
                None => vec![(
                    owner.clone(),
                    format!(
                        "https://{}/{}/_apis/git/pullrequests",
                        self.host,
                        seg(owner)
                    ),
                )],
            },
            Scope::InvolvingMe { .. } => self
                .orgs()
                .into_iter()
                .map(|o| {
                    let u = format!("https://{}/{}/_apis/git/pullrequests", self.host, seg(&o));
                    (o, u)
                })
                .collect(),
        };
        let mine = matches!(scope, Scope::InvolvingMe { .. });
        let mut criteria: Vec<String> = vec![];
        if mine || filter.involvement == Involvement::Authored {
            criteria.push(format!("&searchCriteria.creatorId={my_id}"));
        }
        if mine || filter.involvement == Involvement::Reviewing {
            criteria.push(format!("&searchCriteria.reviewerId={my_id}"));
        }
        if criteria.is_empty() {
            criteria.push(String::new());
        }
        let mut out: Vec<ChangeRequest> = vec![];
        let mut raw: Vec<(ChangeRequest, Value)> = vec![];
        for (org, base) in &bases {
            for c in &criteria {
                let url = format!("{base}?searchCriteria.status={status}{c}&{API}");
                let items = match self.get_paged(&url).await {
                    Ok(v) => v,
                    // An org from a stale remote should not fail the whole inbox.
                    Err(ProviderError::NotFound(_)) if mine => continue,
                    Err(e) => return Err(e),
                };
                for v in items {
                    let cr = parse_pr(&self.host, org, &v, &me);
                    if !out.iter().any(|x| x.id == cr.id) {
                        out.push(cr.clone());
                        raw.push((cr, v));
                    }
                }
            }
        }
        self.remember(&raw);
        out.sort_by_key(|c| std::cmp::Reverse(c.updated_at));
        Ok(out)
    }

    async fn get_change_request(
        &self,
        id: &ChangeRequestId,
    ) -> ProviderResult<ChangeRequestDetail> {
        let me = self.me().await?.clone();
        let base = self.pr_url(id)?;
        let (org, project, _) = split_repo(&id.repo.path)
            .ok_or_else(|| ProviderError::NotFound(id.repo.path.clone()))?;
        let v = self.get(&format!("{base}?{API}")).await?;
        let mut summary = parse_pr(&self.host, org, &v, &me);
        self.remember(&[(summary.clone(), v.clone())]);
        let (tu, cu, su) = (
            format!("{base}/threads?{API}"),
            format!("{base}/commits?$top=250&{API}"),
            format!("{base}/statuses?{API}"),
        );
        let (threads, commits, evals, statuses) = tokio::join!(
            self.get(&tu),
            self.get(&cu),
            self.policy_evaluations(id),
            self.get(&su)
        );
        let threads = threads?["value"].as_array().cloned().unwrap_or_default();
        let commits = commits
            .map(|c| c["value"].as_array().cloned().unwrap_or_default())
            .unwrap_or_default();
        // Policies and statuses may be off limits (permissions); the PR still shows.
        let evals = evals.unwrap_or_default();
        let statuses = statuses
            .map(|s| s["value"].as_array().cloned().unwrap_or_default())
            .unwrap_or_default();
        let mut runs = checks::policy_runs(&evals, org, project);
        runs.extend(checks::status_runs(&statuses));
        summary.checks = checks::summary(&runs);
        let parsed = parse_threads(&threads, &me);
        summary.comments_count = parsed.iter().map(|t| t.comments.len() as u32).sum();
        summary.needs_you = compute_needs_you(&summary, &me.username);
        let my_vote = my_vote(&v, &me);
        let mut review = ReviewInfo {
            caps: azure_capabilities(&summary, my_vote),
            my_state: (my_vote != 0).then(|| vote_state(my_vote)),
            approvals: None,
        };
        let required = checks::required_approvers(&evals);
        if required > 0 {
            let given = v["reviewers"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|r| r["isContainer"] != true && r["vote"].as_i64().unwrap_or(0) >= 5)
                .count() as u32;
            review.approvals = Some(Approvals { given, required });
        }
        let mut events = vec![TimelineEvent {
            kind: TimelineKind::Opened,
            actor: summary.author.clone(),
            at: summary.created_at,
        }];
        events.extend(vote_events(&threads));
        events.extend(close_event(&v));
        events.sort_by_key(|e| e.at);
        let edit = edit::edit_info(&v, &summary, &evals);
        let failed: Vec<String> = checks::failed_builds(&evals);
        let checks = ChecksInfo {
            runs,
            can_rerun: !failed.is_empty() && summary.state.is_active(),
            rerun_target: (!failed.is_empty()).then(|| failed.join(",")),
        };
        Ok(ChangeRequestDetail {
            description: s(&v["description"]),
            threads: parsed,
            head_sha: v["lastMergeSourceCommit"]["commitId"]
                .as_str()
                .map(str::to_string),
            review,
            events,
            edit,
            checks,
            reactions: vec![],
            commits: parse_commits(&commits, &id.repo),
            my_review_sha: None,
            viewed: None,
            node_id: None,
            summary,
        })
    }

    async fn list_repos(&self, owner: &str) -> ProviderResult<Vec<RepoRef>> {
        let (org, project) = owner
            .split_once('/')
            .ok_or_else(|| ProviderError::NotFound(format!("{owner} (expected org/project)")))?;
        let v = self
            .get(&format!(
                "https://{}/{}/{}/_apis/git/repositories?{API}",
                self.host,
                seg(org),
                seg(project)
            ))
            .await?;
        Ok(v["value"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| r["isDisabled"] != true)
            .filter_map(|r| r["name"].as_str())
            .map(|n| {
                RepoRef::new(
                    ProviderKind::Azure,
                    &self.host,
                    format!("{org}/{project}/{n}"),
                )
            })
            .collect())
    }

    async fn add_comment(&self, id: &ChangeRequestId, body: &str) -> ProviderResult<Comment> {
        let t = self
            .call(
                reqwest::Method::POST,
                &format!("{}/threads?{API}", self.pr_url(id)?),
                Some(&edit::thread_payload(body, None)),
            )
            .await?;
        Self::first_comment(&t)
    }

    async fn reply_to_thread(
        &self,
        id: &ChangeRequestId,
        thread_id: &str,
        body: &str,
    ) -> ProviderResult<Comment> {
        let url = format!("{}/threads/{thread_id}/comments?{API}", self.pr_url(id)?);
        let c = self
            .call(
                reqwest::Method::POST,
                &url,
                Some(&edit::reply_payload(body)),
            )
            .await?;
        Ok(comment(thread_id.parse().unwrap_or(0), &c))
    }

    async fn delete_comment(&self, id: &ChangeRequestId, comment_id: &str) -> ProviderResult<()> {
        let url = edit::delete_comment_url(&self.pr_url(id)?, comment_id)?;
        self.call(reqwest::Method::DELETE, &url, None)
            .await
            .map(|_| ())
    }

    async fn edit_comment(
        &self,
        id: &ChangeRequestId,
        comment_id: &str,
        body: &str,
    ) -> ProviderResult<Comment> {
        let (t, c) = split_comment_id(comment_id)?;
        let url = format!("{}/threads/{t}/comments/{c}?{API}", self.pr_url(id)?);
        let v = self
            .call(
                reqwest::Method::PATCH,
                &url,
                Some(&json!({ "content": body })),
            )
            .await?;
        Ok(comment(t, &v))
    }

    async fn add_line_comment(
        &self,
        id: &ChangeRequestId,
        position: &DiffPosition,
        body: &str,
    ) -> ProviderResult<Comment> {
        let t = self
            .call(
                reqwest::Method::POST,
                &format!("{}/threads?{API}", self.pr_url(id)?),
                Some(&edit::thread_payload(body, Some(position))),
            )
            .await?;
        Self::first_comment(&t)
    }

    async fn resolve_thread(
        &self,
        id: &ChangeRequestId,
        thread_id: &str,
        resolved: bool,
    ) -> ProviderResult<()> {
        let url = format!("{}/threads/{thread_id}?{API}", self.pr_url(id)?);
        self.call(
            reqwest::Method::PATCH,
            &url,
            Some(&edit::resolve_payload(resolved)),
        )
        .await
        .map(|_| ())
    }

    async fn submit_review(
        &self,
        id: &ChangeRequestId,
        action: ReviewAction,
        body: Option<&str>,
    ) -> ProviderResult<()> {
        let vote = vote_action(action).ok_or_else(|| {
            ProviderError::Unsupported(format!("{} on Azure DevOps", action.label()))
        })?;
        let me = self.me().await?.clone();
        let my_id = me
            .id
            .clone()
            .ok_or_else(|| ProviderError::Decode("no user id".into()))?;
        let url = format!("{}/reviewers/{my_id}?{API}", self.pr_url(id)?);
        self.call(reqwest::Method::PUT, &url, Some(&edit::vote_payload(vote)))
            .await?;
        if let Some(b) = body {
            self.add_comment(id, b).await?;
        }
        Ok(())
    }

    async fn update_change_request(
        &self,
        id: &ChangeRequestId,
        title: Option<&str>,
        body: Option<&str>,
    ) -> ProviderResult<()> {
        self.call(
            reqwest::Method::PATCH,
            &format!("{}?{API}", self.pr_url(id)?),
            Some(&edit::update_payload(title, body)),
        )
        .await
        .map(|_| ())
    }

    async fn set_people(
        &self,
        id: &ChangeRequestId,
        role: PeopleRole,
        current: &[User],
        desired: &[User],
    ) -> ProviderResult<()> {
        if role != PeopleRole::Reviewers {
            return Err(ProviderError::Unsupported(
                "assignees on Azure DevOps".into(),
            ));
        }
        let base = self.pr_url(id)?;
        let (add, remove) = edit::reviewer_changes(current, desired)?;
        for r in add {
            self.call(
                reqwest::Method::PUT,
                &format!("{base}/reviewers/{r}?{API}"),
                Some(&edit::vote_payload(0)),
            )
            .await?;
        }
        for r in remove {
            self.call(
                reqwest::Method::DELETE,
                &format!("{base}/reviewers/{r}?{API}"),
                None,
            )
            .await?;
        }
        Ok(())
    }

    async fn search_users(
        &self,
        id: &ChangeRequestId,
        query_text: &str,
    ) -> ProviderResult<Vec<User>> {
        let (org, _, _) = split_repo(&id.repo.path)
            .ok_or_else(|| ProviderError::NotFound(id.repo.path.clone()))?;
        let url = format!("https://vssps.dev.azure.com/{}/_apis/identities?searchFilter=General&filterValue={}&{API}", seg(org), query(query_text.trim()));
        let v = self.get(&url).await?;
        Ok(v["value"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(identity_user)
            .take(20)
            .collect())
    }

    async fn set_draft(
        &self,
        id: &ChangeRequestId,
        draft: bool,
        _title: &str,
        _node_id: Option<&str>,
    ) -> ProviderResult<()> {
        self.call(
            reqwest::Method::PATCH,
            &format!("{}?{API}", self.pr_url(id)?),
            Some(&json!({ "isDraft": draft })),
        )
        .await
        .map(|_| ())
    }

    async fn set_open(&self, id: &ChangeRequestId, open: bool) -> ProviderResult<()> {
        self.call(
            reqwest::Method::PATCH,
            &format!("{}?{API}", self.pr_url(id)?),
            Some(&edit::status_payload(open)),
        )
        .await
        .map(|_| ())
    }

    async fn merge(&self, id: &ChangeRequestId, opts: &MergeOptions) -> ProviderResult<()> {
        let url = format!("{}?{API}", self.pr_url(id)?);
        let pr = self.get(&url).await?;
        let head = pr["lastMergeSourceCommit"]["commitId"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let me = self.me().await?.id.clone().unwrap_or_default();
        self.call(
            reqwest::Method::PATCH,
            &url,
            Some(&edit::complete_payload(opts, &head, &me)),
        )
        .await
        .map(|_| ())
    }

    async fn rerun_failed(&self, id: &ChangeRequestId, target: &str) -> ProviderResult<()> {
        let (org, project, _) = split_repo(&id.repo.path)
            .ok_or_else(|| ProviderError::NotFound(id.repo.path.clone()))?;
        for eval in target.split(',').filter(|e| !e.is_empty()) {
            let url = format!(
                "https://{}/{}/{}/_apis/policy/evaluations/{eval}?{API_PREVIEW}",
                self.host,
                seg(org),
                seg(project)
            );
            self.call(reqwest::Method::PATCH, &url, None).await?;
        }
        Ok(())
    }

    async fn react(
        &self,
        id: &ChangeRequestId,
        subject: &str,
        kind: ReactionKind,
        add: bool,
    ) -> ProviderResult<()> {
        if kind != ReactionKind::ThumbsUp || subject.is_empty() {
            return Err(ProviderError::Unsupported(
                "Azure DevOps only has likes on comments".into(),
            ));
        }
        let (t, c) = split_comment_id(subject)?;
        let url = format!(
            "{}/threads/{t}/comments/{c}/likes?{API_PREVIEW}",
            self.pr_url(id)?
        );
        let method = if add {
            reqwest::Method::POST
        } else {
            reqwest::Method::DELETE
        };
        self.call(method, &url, None).await.map(|_| ())
    }

    async fn list_changed_files(&self, id: &ChangeRequestId) -> ProviderResult<Vec<ChangedFile>> {
        self.changed_files(id).await
    }

    async fn commit_files(
        &self,
        id: &ChangeRequestId,
        sha: &str,
    ) -> ProviderResult<Vec<ChangedFile>> {
        self.commit_diff(id, sha).await
    }

    async fn get_file_content(
        &self,
        id: &ChangeRequestId,
        path: &str,
        rev: &str,
    ) -> ProviderResult<Option<String>> {
        let p = format!("/{}", path.trim_start_matches('/'));
        let url = format!("{}/items?path={}&versionDescriptor.version={}&versionDescriptor.versionType=commit&$format=octetStream&{API}", self.repo_url(id)?, query(&p), query(rev));
        Ok(self
            .get_bytes(&url)
            .await?
            .map(|b| String::from_utf8_lossy(&b).into_owned()))
    }
}
