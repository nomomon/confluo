//! Editing pull requests: capabilities, payloads and the REST/GraphQL calls.

use gv_core::*;
use serde_json::{json, Value};

use super::{split_repo, GitHubProvider};

/// Extra detail-query fields for permissions, assignees and merge settings.
pub(crate) const EDIT_FIELDS: &str = r#"
  id viewerCanUpdate mergeable mergeStateStatus
  assignees(first: 20) { nodes { login name avatarUrl } }
  repository { viewerPermission mergeCommitAllowed squashMergeAllowed rebaseMergeAllowed }
"#;

/// Whether the viewer has at least write (or triage) access from `viewerPermission`.
pub(crate) fn access(pr: &Value) -> (bool, bool) {
    let p = pr["repository"]["viewerPermission"].as_str().unwrap_or_default();
    let write = matches!(p, "ADMIN" | "MAINTAIN" | "WRITE");
    (write, write || p == "TRIAGE")
}

pub(crate) fn edit_info(pr: &Value, cr: &ChangeRequest, me: &str) -> EditInfo {
    let (write, triage) = access(pr);
    let own = cr.author.username.eq_ignore_ascii_case(me);
    let can_update = pr["viewerCanUpdate"] == true || own || write;
    let open = cr.state.is_active();
    let repo = &pr["repository"];
    let mut methods = vec![];
    for (flag, m) in [("mergeCommitAllowed", MergeMethod::Merge), ("squashMergeAllowed", MergeMethod::Squash), ("rebaseMergeAllowed", MergeMethod::Rebase)] {
        if repo[flag] != false {
            methods.push(m);
        }
    }
    let blocked = if !write || !open {
        None
    } else if cr.state == ChangeRequestState::Draft {
        Some("Draft pull requests can't be merged".to_string())
    } else if pr["mergeable"] == "CONFLICTING" {
        Some("This branch has conflicts".to_string())
    } else if pr["mergeStateStatus"] == "BLOCKED" {
        Some("Merging is blocked by branch protection".to_string())
    } else if pr["mergeable"] == "UNKNOWN" {
        Some("GitHub is still checking mergeability".to_string())
    } else {
        None
    };
    let merge = write && open && blocked.is_none();
    let caps = EditCapabilities {
        title: can_update,
        body: can_update,
        reviewers: open && (triage || own),
        assignees: triage,
        draft: open && can_update,
        close: open && can_update,
        reopen: cr.state == ChangeRequestState::Closed && can_update,
        merge,
        merge_methods: if merge { methods } else { vec![] },
        merge_blocked: blocked,
        ..Default::default()
    };
    let assignees = pr["assignees"]["nodes"].as_array().into_iter().flatten().map(super::user).collect();
    EditInfo { caps, assignees, node_id: pr["id"].as_str().map(str::to_string) }
}

/// `PATCH /pulls/{n}` body for title and/or description.
pub(crate) fn update_payload(title: Option<&str>, body: Option<&str>) -> Value {
    let mut v = json!({});
    if let Some(t) = title {
        v["title"] = json!(t);
    }
    if let Some(b) = body {
        v["body"] = json!(b);
    }
    v
}

/// Requested reviewers payload, splitting `team:slug` entries into team reviewers.
pub(crate) fn reviewers_payload(names: &[String]) -> Value {
    let (teams, users): (Vec<&String>, Vec<&String>) = names.iter().partition(|n| n.starts_with("team:"));
    json!({"reviewers": users, "team_reviewers": teams.iter().map(|t| &t[5..]).collect::<Vec<_>>()})
}

pub(crate) fn merge_payload(opts: &MergeOptions) -> Value {
    json!({"merge_method": opts.method.as_str()})
}

fn names(users: &[User]) -> Vec<String> {
    users.iter().map(|u| u.username.clone()).collect()
}

impl GitHubProvider {
    fn pr_url(&self, id: &ChangeRequestId, tail: &str) -> ProviderResult<String> {
        let (o, n) = split_repo(&id.repo)?;
        Ok(format!("{}/repos/{o}/{n}/{tail}", self.api))
    }

    pub(crate) async fn do_update(&self, id: &ChangeRequestId, title: Option<&str>, body: Option<&str>) -> ProviderResult<()> {
        let url = self.pr_url(id, &format!("pulls/{}", id.number))?;
        self.send(self.http.patch(url).json(&update_payload(title, body))).await.map(|_| ())
    }

    pub(crate) async fn do_set_people(&self, id: &ChangeRequestId, role: PeopleRole, current: &[User], desired: &[User]) -> ProviderResult<()> {
        let (add, remove) = people_diff(&names(current), &names(desired));
        let (path, payload): (String, fn(&[String]) -> Value) = match role {
            PeopleRole::Reviewers => (format!("pulls/{}/requested_reviewers", id.number), reviewers_payload),
            PeopleRole::Assignees => (format!("issues/{}/assignees", id.number), |n| json!({"assignees": n})),
        };
        let url = self.pr_url(id, &path)?;
        if !add.is_empty() {
            self.send(self.http.post(&url).json(&payload(&add))).await?;
        }
        if !remove.is_empty() {
            self.send(self.http.delete(&url).json(&payload(&remove))).await?;
        }
        Ok(())
    }

    pub(crate) async fn do_search_users(&self, id: &ChangeRequestId, query: &str) -> ProviderResult<Vec<User>> {
        let (o, n) = split_repo(&id.repo)?;
        let q = "query($o: String!, $n: String!, $q: String) { repository(owner: $o, name: $n) {
                   assignableUsers(first: 20, query: $q) { nodes { login name avatarUrl } } } }";
        let d = self.gql(q, json!({"o": o, "n": n, "q": query})).await?;
        Ok(d["repository"]["assignableUsers"]["nodes"].as_array().into_iter().flatten().map(super::user).collect())
    }

    pub(crate) async fn do_set_draft(&self, id: &ChangeRequestId, draft: bool, node_id: Option<&str>) -> ProviderResult<()> {
        let node = match node_id {
            Some(n) => n.to_string(),
            None => {
                let (o, n) = split_repo(&id.repo)?;
                let d = self.gql("query($o: String!, $n: String!, $num: Int!) { repository(owner: $o, name: $n) { pullRequest(number: $num) { id } } }", json!({"o": o, "n": n, "num": id.number})).await?;
                super::s(&d["repository"]["pullRequest"]["id"])
            }
        };
        let m = if draft { "convertPullRequestToDraft" } else { "markPullRequestReadyForReview" };
        self.gql(&format!("mutation($id: ID!) {{ {m}(input: {{pullRequestId: $id}}) {{ pullRequest {{ isDraft }} }} }}"), json!({"id": node})).await?;
        Ok(())
    }

    pub(crate) async fn do_set_open(&self, id: &ChangeRequestId, open: bool) -> ProviderResult<()> {
        let url = self.pr_url(id, &format!("pulls/{}", id.number))?;
        self.send(self.http.patch(url).json(&json!({"state": if open { "open" } else { "closed" }}))).await.map(|_| ())
    }

    pub(crate) async fn do_merge(&self, id: &ChangeRequestId, opts: &MergeOptions) -> ProviderResult<()> {
        let url = self.pr_url(id, &format!("pulls/{}/merge", id.number))?;
        self.send(self.http.put(url).json(&merge_payload(opts))).await.map(|_| ())
    }

    /// Annotations of a failed run, else the tail of its Actions job log.
    pub(crate) async fn do_check_log(&self, id: &ChangeRequestId, run: &CheckRun) -> ProviderResult<Vec<String>> {
        if run.id.is_empty() {
            return Ok(vec![]);
        }
        let notes = self.send(self.http.get(self.pr_url(id, &format!("check-runs/{}/annotations", run.id))?)).await?;
        let lines = super::checks::annotation_lines(&notes);
        if lines.iter().any(|l| !l.starts_with("Process completed with exit code")) {
            return Ok(lines);
        }
        let log = self.get_raw(&self.pr_url(id, &format!("actions/jobs/{}/logs", run.id))?).await?;
        Ok(log.map(|l| log_tail(&l, 15)).unwrap_or(lines))
    }

    pub(crate) async fn do_rerun(&self, id: &ChangeRequestId, target: &str) -> ProviderResult<()> {
        for run in target.split(',').filter(|r| !r.is_empty()) {
            self.send(self.http.post(self.pr_url(id, &format!("actions/runs/{run}/rerun-failed-jobs"))?)).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cr(author: &str, state: &str) -> ChangeRequest {
        let n = json!({"number": 1, "state": state, "author": {"login": author}, "repository": {"nameWithOwner": "a/b"}});
        GitHubProvider::new("github.com", "x".into()).parse_pr(&n, "me").unwrap()
    }

    #[test]
    fn payloads() {
        assert_eq!(update_payload(Some("T"), None), json!({"title": "T"}));
        assert_eq!(reviewers_payload(&["ada".into(), "team:core".into()]), json!({"reviewers": ["ada"], "team_reviewers": ["core"]}));
        let o = MergeOptions { method: MergeMethod::Squash, ..Default::default() };
        assert_eq!(merge_payload(&o), json!({"merge_method": "squash"}));
    }

    #[test]
    fn capabilities() {
        let mine = cr("me", "OPEN");
        let pr = json!({"viewerCanUpdate": true, "mergeable": "MERGEABLE", "mergeStateStatus": "CLEAN",
            "repository": {"viewerPermission": "ADMIN", "mergeCommitAllowed": false, "squashMergeAllowed": true, "rebaseMergeAllowed": true}});
        let e = edit_info(&pr, &mine, "me");
        assert!(e.caps.title && e.caps.draft && e.caps.close && !e.caps.reopen && e.caps.merge);
        assert_eq!(e.caps.merge_methods, vec![MergeMethod::Squash, MergeMethod::Rebase]);
        let read = json!({"viewerCanUpdate": false, "repository": {"viewerPermission": "READ"}});
        let e = edit_info(&read, &cr("ada", "OPEN"), "me");
        assert!(!e.caps.title && !e.caps.assignees && !e.caps.merge && e.caps.merge_blocked.is_none());
        let conflict = json!({"viewerCanUpdate": true, "mergeable": "CONFLICTING", "repository": {"viewerPermission": "WRITE"}});
        let e = edit_info(&conflict, &mine, "me");
        assert!(!e.caps.merge && e.caps.merge_blocked.is_some());
    }
}
