//! GitHub provider: GraphQL for reads, REST/GraphQL mutations for comments.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use gv_core::*;
use serde_json::{json, Value};
use tokio::sync::OnceCell;

mod checks;
mod edit;
mod extras;
mod inline;
mod review;

/// GitHub rejects search queries longer than this.
const MAX_QUERY_LEN: usize = 250;

pub struct GitHubProvider {
    host: String,
    api: String,
    graphql: String,
    token: String,
    http: reqwest::Client,
    viewer: OnceCell<User>,
}

const PR_FIELDS: &str = r#"
  number title url isDraft state createdAt updatedAt headRefName baseRefName
  additions deletions changedFiles totalCommentsCount
  repository { nameWithOwner }
  author { __typename login avatarUrl ... on User { name } }
  labels(first: 20) { nodes { name } }
  reviewRequests(first: 20) { nodes { requestedReviewer { ... on User { login name avatarUrl } ... on Team { slug } } } }
  latestReviews(first: 20) { nodes { state author { login avatarUrl ... on User { name } } } }
  commits(last: 1) { nodes { commit { oid statusCheckRollup { contexts(first: 100) { nodes {
    __typename ... on CheckRun { status conclusion } ... on StatusContext { state } } } } } } }
"#;

impl GitHubProvider {
    pub fn new(host: &str, token: String) -> Self {
        let host = host.to_ascii_lowercase();
        let (api, graphql) = if host == "github.com" {
            ("https://api.github.com".to_string(), "https://api.github.com/graphql".to_string())
        } else {
            (format!("https://{host}/api/v3"), format!("https://{host}/api/graphql"))
        };
        let http = reqwest::Client::builder().user_agent("git-viewer").build().expect("http client");
        Self { host, api, graphql, token, http, viewer: OnceCell::new() }
    }

    async fn send(&self, req: reqwest::RequestBuilder) -> ProviderResult<Value> {
        let resp = gv_http::send(req.bearer_auth(&self.token).header("Accept", "application/vnd.github+json")).await?;
        let (status, body) = (resp.status, resp.text());
        match status {
            200..=299 if body.trim().is_empty() => Ok(Value::Null),
            200..=299 => serde_json::from_str(&body).map_err(|e| ProviderError::Decode(e.to_string())),
            401 => Err(ProviderError::Unauthenticated(self.host.clone())),
            404 => Err(ProviderError::NotFound(body)),
            _ => Err(ProviderError::Http { status, body }),
        }
    }

    /// GET returning raw text (`Accept: raw`); None on 404.
    async fn get_raw(&self, url: &str) -> ProviderResult<Option<String>> {
        let resp = gv_http::send(self.http.get(url).bearer_auth(&self.token).header("Accept", "application/vnd.github.raw")).await?;
        let (status, body) = (resp.status, resp.text());
        match status {
            200..=299 => Ok(Some(body)),
            404 => Ok(None),
            401 => Err(ProviderError::Unauthenticated(self.host.clone())),
            _ => Err(ProviderError::Http { status, body }),
        }
    }

    async fn gql(&self, query: &str, variables: Value) -> ProviderResult<Value> {
        let v = self.send(self.http.post(&self.graphql).json(&json!({"query": query, "variables": variables}))).await?;
        if let Some(errs) = v.get("errors").and_then(|e| e.as_array()).filter(|e| !e.is_empty()) {
            let msg = errs.iter().filter_map(|e| e["message"].as_str()).collect::<Vec<_>>().join("; ");
            if errs.iter().any(|e| e["type"] == "NOT_FOUND") {
                return Err(ProviderError::NotFound(msg));
            }
            return Err(ProviderError::Http { status: 200, body: msg });
        }
        Ok(v["data"].clone())
    }

    async fn search(&self, q: &str, me: &str) -> ProviderResult<Vec<ChangeRequest>> {
        let query = format!(
            "query($q: String!, $after: String) {{ search(query: $q, type: ISSUE, first: 50, after: $after) {{
               pageInfo {{ hasNextPage endCursor }} nodes {{ ... on PullRequest {{ {PR_FIELDS} }} }} }}
               rateLimit {{ cost remaining resetAt }} }}"
        );
        let mut out = vec![];
        let mut after = Value::Null;
        for _ in 0..4 {
            let d = self.gql(&query, json!({"q": q, "after": after})).await?;
            if gv_http::debug_enabled() {
                let r = &d["rateLimit"];
                eprintln!("github search cost {} remaining {} reset {}", r["cost"], r["remaining"], r["resetAt"]);
            }
            for n in d["search"]["nodes"].as_array().into_iter().flatten() {
                if n.get("number").is_some() {
                    out.push(self.parse_pr(n, me)?);
                }
            }
            if d["search"]["pageInfo"]["hasNextPage"] != true {
                break;
            }
            after = d["search"]["pageInfo"]["endCursor"].clone();
        }
        Ok(out)
    }

    fn parse_pr(&self, n: &Value, me: &str) -> ProviderResult<ChangeRequest> {
        let repo_path = n["repository"]["nameWithOwner"].as_str().unwrap_or_default();
        let state = match n["state"].as_str() {
            Some("MERGED") => ChangeRequestState::Merged,
            Some("CLOSED") => ChangeRequestState::Closed,
            _ if n["isDraft"] == true => ChangeRequestState::Draft,
            _ => ChangeRequestState::Open,
        };
        let mut reviewers: Vec<Review> = vec![];
        for r in n["latestReviews"]["nodes"].as_array().into_iter().flatten() {
            let st = match r["state"].as_str() {
                Some("APPROVED") => ReviewState::Approved,
                Some("CHANGES_REQUESTED") => ReviewState::ChangesRequested,
                Some("PENDING") => ReviewState::Pending,
                _ => ReviewState::Commented,
            };
            reviewers.push(Review { user: user(&r["author"]), state: st });
        }
        for r in n["reviewRequests"]["nodes"].as_array().into_iter().flatten() {
            let rr = &r["requestedReviewer"];
            let u = if let Some(slug) = rr["slug"].as_str() { User::named(format!("team:{slug}")) } else { user(rr) };
            // A re-request after a review means the reviewer owes a new review.
            if let Some(existing) = reviewers.iter_mut().find(|x| x.user.username == u.username) {
                existing.state = ReviewState::Pending;
            } else {
                reviewers.push(Review { user: u, state: ReviewState::Pending });
            }
        }
        let mut checks = CheckSummary::default();
        let commit = &n["commits"]["nodes"][0]["commit"];
        for c in commit["statusCheckRollup"]["contexts"]["nodes"].as_array().into_iter().flatten() {
            checks.total += 1;
            let outcome = if c["__typename"] == "CheckRun" {
                if c["status"] != "COMPLETED" {
                    "pending"
                } else {
                    match c["conclusion"].as_str() {
                        Some("SUCCESS" | "NEUTRAL" | "SKIPPED") => "passed",
                        _ => "failed",
                    }
                }
            } else {
                match c["state"].as_str() {
                    Some("SUCCESS") => "passed",
                    Some("PENDING" | "EXPECTED") => "pending",
                    _ => "failed",
                }
            };
            match outcome {
                "passed" => checks.passed += 1,
                "pending" => checks.pending += 1,
                _ => checks.failed += 1,
            }
        }
        let mut cr = ChangeRequest {
            id: ChangeRequestId { repo: RepoRef::new(ProviderKind::GitHub, &self.host, repo_path), number: n["number"].as_u64().unwrap_or(0) },
            title: s(&n["title"]),
            author: user(&n["author"]),
            state,
            source_branch: s(&n["headRefName"]),
            target_branch: s(&n["baseRefName"]),
            reviewers,
            checks,
            labels: n["labels"]["nodes"].as_array().into_iter().flatten().map(|l| s(&l["name"])).collect(),
            comments_count: n["totalCommentsCount"].as_u64().unwrap_or(0) as u32,
            additions: n["additions"].as_u64().map(|x| x as u32),
            deletions: n["deletions"].as_u64().map(|x| x as u32),
            changed_files: n["changedFiles"].as_u64().map(|x| x as u32),
            updated_at: time(&n["updatedAt"]),
            created_at: time(&n["createdAt"]),
            web_url: s(&n["url"]),
            needs_you: false,
        };
        cr.needs_you = compute_needs_you(&cr, me);
        Ok(cr)
    }

    async fn me(&self) -> ProviderResult<String> {
        Ok(self.whoami().await?.username)
    }

    /// One search per chunk of repos (`repo:a repo:b ...` is an OR), split back per repo.
    async fn list_repos_batched(&self, repos: Vec<RepoRef>, filter: &Filter) -> Vec<(Scope, ProviderResult<Vec<ChangeRequest>>)> {
        let me = match self.me().await {
            Ok(m) => m,
            Err(e) => {
                let msg = e.to_string();
                return repos.into_iter().map(|r| (Scope::Repo(r), Err(ProviderError::Network(msg.clone())))).collect();
            }
        };
        let base = format!("is:pr archived:false{}", state_qualifier(filter.state));
        let mut out = vec![];
        for chunk in chunk_repos(&repos, MAX_QUERY_LEN - base.len()) {
            let q = format!("{base} {}", chunk.iter().map(|r| format!("repo:{}", r.path)).collect::<Vec<_>>().join(" "));
            let res = self.search(&q, &me).await;
            for r in chunk {
                let scoped = match &res {
                    Ok(items) => Ok(items.iter().filter(|c| c.id.repo.path.eq_ignore_ascii_case(&r.path)).cloned().collect()),
                    Err(ProviderError::RateLimited { host, until }) => Err(ProviderError::RateLimited { host: host.clone(), until: *until }),
                    Err(e) => Err(ProviderError::Network(e.to_string())),
                };
                out.push((Scope::Repo(r.clone()), scoped));
            }
        }
        out
    }
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn time(v: &Value) -> DateTime<Utc> {
    v.as_str().and_then(|t| DateTime::parse_from_rfc3339(t).ok()).map(|t| t.with_timezone(&Utc)).unwrap_or_default()
}

fn user(v: &Value) -> User {
    let username = v["login"].as_str().unwrap_or("ghost").to_string();
    User {
        bot: v["__typename"] == "Bot" || username.ends_with("[bot]"),
        id: None,
        username,
        name: v["name"].as_str().filter(|x| !x.is_empty()).map(str::to_string),
        avatar_url: v["avatarUrl"].as_str().map(str::to_string),
    }
}

fn state_qualifier(s: StateFilter) -> &'static str {
    match s {
        StateFilter::Active => " is:open",
        StateFilter::Merged => " is:merged",
        StateFilter::Closed => " is:closed is:unmerged",
        StateFilter::Any => "",
    }
}

/// Split repos into groups whose `repo:` qualifiers fit in `budget` characters.
fn chunk_repos(repos: &[RepoRef], budget: usize) -> Vec<Vec<&RepoRef>> {
    let mut out: Vec<Vec<&RepoRef>> = vec![];
    let mut len = 0;
    for r in repos {
        let add = r.path.len() + 6;
        if out.is_empty() || len + add > budget {
            out.push(vec![]);
            len = 0;
        }
        out.last_mut().expect("pushed").push(r);
        len += add;
    }
    out
}

fn comment(v: &Value) -> Comment {
    Comment {
        id: s(&v["id"]),
        author: user(&v["author"]),
        body: s(&v["body"]),
        created_at: time(&v["createdAt"]),
        position: v["path"].as_str().map(|p| InlinePosition {
            path: p.to_string(),
            line: v["line"].as_u64().or_else(|| v["originalLine"].as_u64()).map(|l| l as u32),
        }),
        reactions: extras::reactions(v),
    }
}

/// A comment from a REST response (issue or review comment).
fn rest_comment(v: &Value) -> Comment {
    Comment {
        id: s(&v["node_id"]),
        author: User {
            username: s(&v["user"]["login"]),
            name: None,
            avatar_url: v["user"]["avatar_url"].as_str().map(str::to_string),
            bot: v["user"]["type"] == "Bot",
            id: None,
        },
        body: s(&v["body"]),
        created_at: time(&v["created_at"]),
        position: v["path"].as_str().map(|p| InlinePosition { path: p.into(), line: v["line"].as_u64().map(|l| l as u32) }),
        reactions: vec![],
    }
}

/// One entry of `GET /pulls/{n}/files`.
fn changed_file(v: &Value) -> ChangedFile {
    let status = match v["status"].as_str().unwrap_or_default() {
        "added" => FileStatus::Added,
        "removed" => FileStatus::Deleted,
        "renamed" => FileStatus::Renamed,
        _ => FileStatus::Modified,
    };
    let new_path = s(&v["filename"]);
    let old_path = v["previous_filename"].as_str().map(str::to_string).unwrap_or_else(|| new_path.clone());
    let patch = v["patch"].as_str();
    let mut f = ChangedFile::from_patch(&old_path, &new_path, status, patch.unwrap_or_default());
    f.additions = v["additions"].as_u64().unwrap_or(0) as u32;
    f.deletions = v["deletions"].as_u64().unwrap_or(0) as u32;
    if patch.is_none() {
        // GitHub omits the patch for binaries and very large diffs.
        let changes = v["changes"].as_u64().unwrap_or(0);
        f.binary = changes == 0 && status != FileStatus::Renamed;
        f.too_large = changes > 0;
    }
    f
}

fn urlencoding_seg(p: &str) -> String {
    p.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn split_repo(r: &RepoRef) -> ProviderResult<(&str, &str)> {
    r.path.split_once('/').ok_or_else(|| ProviderError::NotFound(r.path.clone()))
}

fn search_qualifiers(scope: &Scope) -> String {
    match scope {
        Scope::Repo(r) => format!("repo:{}", r.path),
        Scope::Group { owner, .. } => format!("user:{owner}"),
        Scope::InvolvingMe { .. } => "involves:@me".into(),
    }
}

#[async_trait]
impl Provider for GitHubProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::GitHub
    }

    fn host(&self) -> &str {
        &self.host
    }

    async fn whoami(&self) -> ProviderResult<User> {
        let u = self
            .viewer
            .get_or_try_init(|| async { Ok::<_, ProviderError>(user(&self.gql("query { viewer { login name avatarUrl } }", json!({})).await?["viewer"])) })
            .await?;
        Ok(u.clone())
    }

    async fn list_many(&self, scopes: Vec<Scope>, filter: &Filter) -> Vec<(Scope, ProviderResult<Vec<ChangeRequest>>)> {
        let (repos, other): (Vec<Scope>, Vec<Scope>) = scopes.into_iter().partition(|s| matches!(s, Scope::Repo(_)) && filter.involvement == Involvement::All);
        let mut out = vec![];
        for s in other {
            let r = self.list_change_requests(&s, filter).await;
            out.push((s, r));
        }
        let repos = repos.into_iter().filter_map(|s| if let Scope::Repo(r) = s { Some(r) } else { None }).collect::<Vec<_>>();
        if !repos.is_empty() {
            out.extend(self.list_repos_batched(repos, filter).await);
        }
        out
    }

    async fn list_change_requests(&self, scope: &Scope, filter: &Filter) -> ProviderResult<Vec<ChangeRequest>> {
        let me = self.me().await?;
        let state = state_qualifier(filter.state);
        let base = format!("is:pr archived:false {}{state}", search_qualifiers(scope));
        let queries: Vec<String> = match filter.involvement {
            Involvement::Authored => vec![format!("{base} author:@me")],
            Involvement::Reviewing => vec![format!("{base} review-requested:@me"), format!("{base} reviewed-by:@me -author:@me")],
            Involvement::All => vec![base],
        };
        let mut out: Vec<ChangeRequest> = vec![];
        for q in queries {
            for cr in self.search(&q, &me).await? {
                if !out.iter().any(|x| x.id == cr.id) {
                    out.push(cr);
                }
            }
        }
        out.sort_by_key(|c| std::cmp::Reverse(c.updated_at));
        Ok(out)
    }

    async fn get_change_request(&self, id: &ChangeRequestId) -> ProviderResult<ChangeRequestDetail> {
        let (owner, name) = split_repo(&id.repo)?;
        let me = self.me().await?;
        let query = format!(
            "query($o: String!, $n: String!, $num: Int!) {{ repository(owner: $o, name: $n) {{ pullRequest(number: $num) {{
               {PR_FIELDS} {} {} {} {R} body mergedAt closedAt mergedBy {{ login avatarUrl }}
               comments(first: 100) {{ nodes {{ id body createdAt author {{ login avatarUrl ... on User {{ name }} }} {R} }} }}
               reviews(first: 50) {{ nodes {{ id state body createdAt commit {{ oid }} author {{ login avatarUrl ... on User {{ name }} }} {R} }} }}
               reviewThreads(first: 100) {{ nodes {{ {} }} }}
             }} }} }}",
            edit::EDIT_FIELDS,
            checks::CHECK_FIELDS,
            extras::PR_EXTRA_FIELDS,
            inline::THREAD_FIELDS,
            R = extras::REACTION_FIELDS,
        );
        let d = self.gql(&query, json!({"o": owner, "n": name, "num": id.number})).await?;
        let pr = &d["repository"]["pullRequest"];
        if pr.is_null() {
            return Err(ProviderError::NotFound(id.key()));
        }
        let summary = self.parse_pr(pr, &me)?;
        let mut threads: Vec<Thread> = vec![];
        // Top-level conversation comments and review summaries become single-comment threads.
        for c in pr["comments"]["nodes"].as_array().into_iter().flatten() {
            threads.push(Thread::conversation(s(&c["id"]), vec![comment(c)]));
        }
        for c in pr["reviews"]["nodes"].as_array().into_iter().flatten() {
            if !c["body"].as_str().unwrap_or_default().is_empty() {
                threads.push(Thread::conversation(s(&c["id"]), vec![comment(c)]));
            }
        }
        for t in pr["reviewThreads"]["nodes"].as_array().into_iter().flatten() {
            threads.push(inline::parse_thread(t));
        }
        threads.sort_by_key(|t| t.comments.first().map(|c| c.created_at));
        let events = review::timeline(pr, &summary);
        let review = ReviewInfo { caps: github_capabilities(&summary, &me), my_state: my_review_state(&summary, &me), approvals: None };
        let edit = edit::edit_info(pr, &summary, &me);
        let checks = checks::checks_info(pr, edit::access(pr).0);
        Ok(ChangeRequestDetail {
            review,
            events,
            edit,
            checks,
            summary,
            description: s(&pr["body"]),
            threads,
            head_sha: pr["commits"]["nodes"][0]["commit"]["oid"].as_str().map(str::to_string),
            reactions: extras::reactions(pr),
            commits: extras::commits(pr),
            my_review_sha: extras::my_review_sha(pr, &me),
            viewed: extras::viewed(pr),
            node_id: pr["id"].as_str().map(str::to_string),
        })
    }

    async fn list_repos(&self, owner: &str) -> ProviderResult<Vec<RepoRef>> {
        let mut out = vec![];
        for page in 1..=10 {
            let url = format!("{}/users/{owner}/repos?per_page=100&page={page}&type=all", self.api);
            let v = self.send(self.http.get(url)).await?;
            let arr = v.as_array().cloned().unwrap_or_default();
            for r in &arr {
                if r["archived"] != true {
                    out.push(RepoRef::new(ProviderKind::GitHub, &self.host, s(&r["full_name"])));
                }
            }
            if arr.len() < 100 {
                break;
            }
        }
        Ok(out)
    }

    async fn add_comment(&self, id: &ChangeRequestId, body: &str) -> ProviderResult<Comment> {
        let (o, n) = split_repo(&id.repo)?;
        let url = format!("{}/repos/{o}/{n}/issues/{}/comments", self.api, id.number);
        let v = self.send(self.http.post(url).json(&json!({"body": body}))).await?;
        Ok(rest_comment(&v))
    }

    async fn reply_to_thread(&self, id: &ChangeRequestId, thread_id: &str, body: &str) -> ProviderResult<Comment> {
        // Only review threads (PRRT_) support replies; conversation comments are flat.
        if !thread_id.starts_with("PRRT_") {
            return self.add_comment(id, body).await;
        }
        let d = self
            .gql(
                "mutation($t: ID!, $b: String!) { addPullRequestReviewThreadReply(input: {pullRequestReviewThreadId: $t, body: $b}) {
                   comment { id body createdAt path line author { login avatarUrl } } } }",
                json!({"t": thread_id, "b": body}),
            )
            .await?;
        Ok(comment(&d["addPullRequestReviewThreadReply"]["comment"]))
    }

    async fn add_line_comment(&self, id: &ChangeRequestId, position: &DiffPosition, body: &str) -> ProviderResult<Comment> {
        let (o, n) = split_repo(&id.repo)?;
        let head = match &position.head_sha {
            Some(h) => h.clone(),
            None => s(&self.send(self.http.get(format!("{}/repos/{o}/{n}/pulls/{}", self.api, id.number))).await?["head"]["sha"]),
        };
        let url = format!("{}/repos/{o}/{n}/pulls/{}/comments", self.api, id.number);
        let v = self.send(self.http.post(url).json(&inline::line_comment_payload(position, &head, body))).await?;
        Ok(rest_comment(&v))
    }

    async fn resolve_thread(&self, _id: &ChangeRequestId, thread_id: &str, resolved: bool) -> ProviderResult<()> {
        let m = if resolved { "resolveReviewThread" } else { "unresolveReviewThread" };
        let q = format!("mutation($t: ID!) {{ {m}(input: {{threadId: $t}}) {{ thread {{ id isResolved }} }} }}");
        self.gql(&q, json!({"t": thread_id})).await?;
        Ok(())
    }

    async fn list_changed_files(&self, id: &ChangeRequestId) -> ProviderResult<Vec<ChangedFile>> {
        let (owner, name) = split_repo(&id.repo)?;
        let mut out = vec![];
        for page in 1..=30 {
            let url = format!("{}/repos/{owner}/{name}/pulls/{}/files?per_page=100&page={page}", self.api, id.number);
            let v = self.send(self.http.get(url)).await?;
            let items = v.as_array().cloned().unwrap_or_default();
            out.extend(items.iter().map(changed_file));
            if items.len() < 100 {
                break;
            }
        }
        Ok(out)
    }

    async fn get_file_content(&self, id: &ChangeRequestId, path: &str, rev: &str) -> ProviderResult<Option<String>> {
        let (owner, name) = split_repo(&id.repo)?;
        let path: String = path.split('/').map(urlencoding_seg).collect::<Vec<_>>().join("/");
        self.get_raw(&format!("{}/repos/{owner}/{name}/contents/{path}?ref={rev}", self.api)).await
    }

    async fn submit_review(&self, id: &ChangeRequestId, action: ReviewAction, body: Option<&str>) -> ProviderResult<()> {
        let payload = review::review_payload(action, body).ok_or_else(|| ProviderError::Unsupported(format!("{} on GitHub", action.label())))?;
        let (o, n) = split_repo(&id.repo)?;
        let url = format!("{}/repos/{o}/{n}/pulls/{}/reviews", self.api, id.number);
        self.send(self.http.post(url).json(&payload)).await?;
        Ok(())
    }

    async fn update_change_request(&self, id: &ChangeRequestId, title: Option<&str>, body: Option<&str>) -> ProviderResult<()> {
        self.do_update(id, title, body).await
    }

    async fn set_people(&self, id: &ChangeRequestId, role: PeopleRole, current: &[User], desired: &[User]) -> ProviderResult<()> {
        self.do_set_people(id, role, current, desired).await
    }

    async fn search_users(&self, id: &ChangeRequestId, query: &str) -> ProviderResult<Vec<User>> {
        self.do_search_users(id, query).await
    }

    async fn set_draft(&self, id: &ChangeRequestId, draft: bool, _title: &str, node_id: Option<&str>) -> ProviderResult<()> {
        self.do_set_draft(id, draft, node_id).await
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

    async fn rerun_failed(&self, id: &ChangeRequestId, target: &str) -> ProviderResult<()> {
        self.do_rerun(id, target).await
    }

    async fn react(&self, _id: &ChangeRequestId, subject: &str, kind: ReactionKind, add: bool) -> ProviderResult<()> {
        self.do_react(subject, kind, add).await
    }

    async fn set_file_viewed(&self, _id: &ChangeRequestId, pr_node: Option<&str>, path: &str, viewed: bool) -> ProviderResult<()> {
        let node = pr_node.ok_or_else(|| ProviderError::Unsupported("viewed files without a PR id".into()))?;
        self.do_set_viewed(node, path, viewed).await
    }

    async fn edit_comment(&self, _id: &ChangeRequestId, comment_id: &str, body: &str) -> ProviderResult<Comment> {
        let (mutation, field) = if comment_id.starts_with("PRRC_") {
            ("updatePullRequestReviewComment(input: {pullRequestReviewCommentId: $c, body: $b}) { pullRequestReviewComment { id body createdAt path line author { login avatarUrl } } }", "pullRequestReviewComment")
        } else if comment_id.starts_with("PRR_") {
            ("updatePullRequestReview(input: {pullRequestReviewId: $c, body: $b}) { pullRequestReview { id body createdAt author { login avatarUrl } } }", "pullRequestReview")
        } else {
            ("updateIssueComment(input: {id: $c, body: $b}) { issueComment { id body createdAt author { login avatarUrl } } }", "issueComment")
        };
        let d = self.gql(&format!("mutation($c: ID!, $b: String!) {{ {mutation} }}"), json!({"c": comment_id, "b": body})).await?;
        let root = d.as_object().and_then(|o| o.values().next()).cloned().unwrap_or_default();
        Ok(comment(&root[field]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_changed_file() {
        let f = changed_file(&json!({"filename": "b.rs", "previous_filename": "a.rs", "status": "renamed",
            "additions": 1, "deletions": 0, "changes": 1, "patch": "@@ -1 +1,2 @@\n a\n+b"}));
        assert_eq!((f.old_path.as_str(), f.status, f.additions), ("a.rs", FileStatus::Renamed, 1));
        assert_eq!(f.hunks[0].lines.len(), 2);
        let big = changed_file(&json!({"filename": "x", "status": "modified", "changes": 9000}));
        assert!(big.too_large && !big.binary);
        let bin = changed_file(&json!({"filename": "x.png", "status": "added", "changes": 0}));
        assert!(bin.binary);
    }

    #[test]
    fn chunks_repo_qualifiers() {
        let repos: Vec<RepoRef> = (0..30).map(|i| RepoRef::new(ProviderKind::GitHub, "github.com", format!("org/repo-{i}"))).collect();
        let chunks = chunk_repos(&repos, 200);
        assert!(chunks.len() > 1);
        assert_eq!(chunks.iter().map(Vec::len).sum::<usize>(), 30);
        for c in &chunks {
            assert!(c.iter().map(|r| r.path.len() + 6).sum::<usize>() <= 200);
        }
    }

    #[test]
    fn parses_pr_node() {
        let p = GitHubProvider::new("github.com", "x".into());
        assert!(user(&json!({"__typename": "Bot", "login": "renovate"})).bot);
        assert!(!user(&json!({"__typename": "User", "login": "bob"})).bot);
        let n = json!({
            "number": 4, "title": "T", "url": "https://github.com/a/b/pull/4", "isDraft": false, "state": "OPEN",
            "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-02T00:00:00Z",
            "headRefName": "f", "baseRefName": "main", "additions": 3, "deletions": 1, "changedFiles": 2, "totalCommentsCount": 5,
            "repository": {"nameWithOwner": "a/b"}, "author": {"login": "bob"},
            "labels": {"nodes": [{"name": "bug"}]},
            "reviewRequests": {"nodes": [{"requestedReviewer": {"login": "me"}}]},
            "latestReviews": {"nodes": [{"state": "APPROVED", "author": {"login": "carol"}}]},
            "commits": {"nodes": [{"commit": {"oid": "abc", "statusCheckRollup": {"contexts": {"nodes": [
                {"__typename": "CheckRun", "status": "COMPLETED", "conclusion": "FAILURE"},
                {"__typename": "StatusContext", "state": "SUCCESS"},
                {"__typename": "CheckRun", "status": "IN_PROGRESS", "conclusion": null}]}}}}]}
        });
        let cr = p.parse_pr(&n, "me").unwrap();
        assert_eq!(cr.id.repo.path, "a/b");
        assert_eq!(cr.checks, CheckSummary { total: 3, passed: 1, failed: 1, pending: 1 });
        assert_eq!(cr.reviewers.len(), 2);
        assert!(cr.needs_you);
        assert_eq!(cr.labels, vec!["bug"]);
    }
}
