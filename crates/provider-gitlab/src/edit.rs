//! Editing merge requests: capabilities from the MR payload, request bodies and calls.

use gv_core::*;
use reqwest::Method;
use serde_json::{json, Value};

use super::{enc, GitLabProvider};

/// Why GitLab won't merge now, from `detailed_merge_status`.
fn blocked_reason(status: &str) -> String {
    match status {
        "conflict" | "broken_status" => "This branch has conflicts".into(),
        "draft_status" => "Draft merge requests can't be merged".into(),
        "not_approved" => "Approvals are still required".into(),
        "discussions_not_resolved" => "Some threads are unresolved".into(),
        "ci_must_pass" => "The pipeline must succeed first".into(),
        "need_rebase" => "The branch needs a rebase".into(),
        "checking" | "unchecked" | "preparing" => "GitLab is still checking mergeability".into(),
        s => format!("Not mergeable ({})", s.replace('_', " ")),
    }
}

/// Edit info from the single-MR payload. Developers (`can_merge`) and the author may edit.
pub(crate) fn edit_info(v: &Value, cr: &ChangeRequest, me: &str) -> EditInfo {
    let can_merge = v["user"]["can_merge"] == true;
    let own = cr.author.username.eq_ignore_ascii_case(me);
    let open = cr.state.is_active();
    let mut caps = if can_merge || own {
        EditCapabilities::editor(open)
    } else {
        EditCapabilities::default()
    };
    caps.reopen = caps.reopen && cr.state == ChangeRequestState::Closed;
    let dms = v["detailed_merge_status"]
        .as_str()
        .or(v["merge_status"].as_str())
        .unwrap_or_default();
    let running = matches!(
        v["head_pipeline"]["status"].as_str(),
        Some("running" | "pending" | "created" | "preparing" | "waiting_for_resource")
    );
    let now = matches!(dms, "mergeable" | "can_be_merged");
    let auto = running
        && matches!(
            dms,
            "ci_still_running" | "ci_must_pass" | "mergeable" | "can_be_merged"
        );
    if can_merge && open {
        if now || auto {
            caps.merge = true;
            caps.merge_methods = vec![MergeMethod::Merge];
            caps.squash_option = true;
            caps.squash_default = v["squash"] == true || v["squash_on_merge"] == true;
            caps.delete_branch_option = true;
            caps.delete_branch_default =
                v["force_remove_source_branch"] == true || v["should_remove_source_branch"] == true;
            caps.auto_merge = auto;
        } else {
            caps.merge_blocked = Some(blocked_reason(dms));
        }
    }
    let assignees = v["assignees"]
        .as_array()
        .into_iter()
        .flatten()
        .map(super::user)
        .collect();
    EditInfo {
        caps,
        assignees,
        node_id: None,
    }
}

/// `PUT /merge_requests/:iid` body setting reviewers or assignees by id.
pub(crate) fn people_payload(role: PeopleRole, desired: &[User]) -> ProviderResult<Value> {
    let mut ids = vec![];
    for u in desired {
        let id =
            u.id.as_deref()
                .and_then(|i| i.parse::<u64>().ok())
                .ok_or_else(|| ProviderError::Decode(format!("no user id for @{}", u.username)))?;
        ids.push(id);
    }
    if ids.is_empty() {
        // GitLab ignores an empty array; 0 clears the list.
        ids.push(0);
    }
    let key = match role {
        PeopleRole::Reviewers => "reviewer_ids",
        PeopleRole::Assignees => "assignee_ids",
    };
    Ok(json!({ key: ids }))
}

pub(crate) fn update_payload(title: Option<&str>, body: Option<&str>) -> Value {
    let mut v = json!({});
    if let Some(t) = title {
        v["title"] = json!(t);
    }
    if let Some(b) = body {
        v["description"] = json!(b);
    }
    v
}

pub(crate) fn merge_payload(o: &MergeOptions) -> Value {
    json!({
        "squash": o.squash || o.method == MergeMethod::Squash,
        "should_remove_source_branch": o.delete_branch,
        "merge_when_pipeline_succeeds": o.when_pipeline_succeeds,
    })
}

pub(crate) fn state_payload(open: bool) -> Value {
    json!({"state_event": if open { "reopen" } else { "close" }})
}

impl GitLabProvider {
    async fn put_mr(&self, id: &ChangeRequestId, body: &Value) -> ProviderResult<()> {
        self.call(Method::PUT, &Self::mr_path(id), Some(body))
            .await
            .map(|_| ())
    }

    pub(crate) async fn do_update(
        &self,
        id: &ChangeRequestId,
        title: Option<&str>,
        body: Option<&str>,
    ) -> ProviderResult<()> {
        self.put_mr(id, &update_payload(title, body)).await
    }

    pub(crate) async fn do_set_people(
        &self,
        id: &ChangeRequestId,
        role: PeopleRole,
        desired: &[User],
    ) -> ProviderResult<()> {
        self.put_mr(id, &people_payload(role, desired)?).await
    }

    pub(crate) async fn do_search_users(
        &self,
        id: &ChangeRequestId,
        query: &str,
    ) -> ProviderResult<Vec<User>> {
        let q: String = query
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '))
            .collect::<String>()
            .replace(' ', "%20");
        let v = self
            .get(&format!(
                "/projects/{}/members/all?query={q}&per_page=20",
                enc(&id.repo.path)
            ))
            .await?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .filter(|m| m["state"] != "blocked")
            .map(super::user)
            .collect())
    }

    pub(crate) async fn do_set_draft(
        &self,
        id: &ChangeRequestId,
        draft: bool,
        title: &str,
    ) -> ProviderResult<()> {
        self.put_mr(id, &json!({"title": draft_title(title, draft)}))
            .await
    }

    pub(crate) async fn do_set_open(&self, id: &ChangeRequestId, open: bool) -> ProviderResult<()> {
        self.put_mr(id, &state_payload(open)).await
    }

    pub(crate) async fn do_merge(
        &self,
        id: &ChangeRequestId,
        opts: &MergeOptions,
    ) -> ProviderResult<()> {
        self.call(
            Method::PUT,
            &format!("{}/merge", Self::mr_path(id)),
            Some(&merge_payload(opts)),
        )
        .await
        .map(|_| ())
    }

    pub(crate) async fn do_check_log(
        &self,
        id: &ChangeRequestId,
        run: &CheckRun,
    ) -> ProviderResult<Vec<String>> {
        if run.id.is_empty() {
            return Ok(vec![]);
        }
        let log = self
            .get_text(&format!(
                "/projects/{}/jobs/{}/trace",
                enc(&id.repo.path),
                run.id
            ))
            .await?;
        Ok(log.map(|l| log_tail(&l, 15)).unwrap_or_default())
    }

    pub(crate) async fn do_rerun(&self, target: &str) -> ProviderResult<()> {
        let (proj, pid) = target
            .split_once('/')
            .ok_or_else(|| ProviderError::Decode(format!("bad pipeline {target}")))?;
        self.call(
            Method::POST,
            &format!("/projects/{proj}/pipelines/{pid}/retry"),
            None,
        )
        .await
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mr(extra: Value) -> (Value, ChangeRequest) {
        let mut v = json!({"iid": 1, "author": {"username": "ada"}, "state": "opened", "web_url": "https://gitlab.com/g/p/-/merge_requests/1",
            "assignees": [{"id": 5, "username": "me"}]});
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let cr = crate::parse_mr("gitlab.com", &v);
        (v, cr)
    }

    #[test]
    fn payloads() {
        let u = |n: &str, id: Option<&str>| User {
            id: id.map(str::to_string),
            ..User::named(n)
        };
        assert_eq!(
            people_payload(
                PeopleRole::Reviewers,
                &[u("a", Some("3")), u("b", Some("4"))]
            )
            .unwrap(),
            json!({"reviewer_ids": [3, 4]})
        );
        assert_eq!(
            people_payload(PeopleRole::Assignees, &[]).unwrap(),
            json!({"assignee_ids": [0]})
        );
        assert!(people_payload(PeopleRole::Assignees, &[u("x", None)]).is_err());
        assert_eq!(
            update_payload(Some("T"), Some("B")),
            json!({"title": "T", "description": "B"})
        );
        assert_eq!(state_payload(false), json!({"state_event": "close"}));
        let o = MergeOptions {
            squash: true,
            delete_branch: true,
            when_pipeline_succeeds: true,
            ..Default::default()
        };
        assert_eq!(
            merge_payload(&o),
            json!({"squash": true, "should_remove_source_branch": true, "merge_when_pipeline_succeeds": true})
        );
    }

    #[test]
    fn capabilities() {
        let (v, cr) = mr(
            json!({"user": {"can_merge": true}, "detailed_merge_status": "mergeable", "squash": true}),
        );
        let e = edit_info(&v, &cr, "me");
        assert!(
            e.caps.title
                && e.caps.close
                && !e.caps.reopen
                && e.caps.merge
                && e.caps.squash_default
                && !e.caps.auto_merge
        );
        assert_eq!(e.assignees[0].id.as_deref(), Some("5"));
        let (v, cr) = mr(
            json!({"user": {"can_merge": true}, "detailed_merge_status": "ci_still_running", "head_pipeline": {"status": "running"}}),
        );
        assert!(edit_info(&v, &cr, "me").caps.auto_merge);
        let (v, cr) = mr(json!({"user": {"can_merge": true}, "detailed_merge_status": "conflict"}));
        let e = edit_info(&v, &cr, "me");
        assert!(
            !e.caps.merge && e.caps.merge_blocked.as_deref() == Some("This branch has conflicts")
        );
        let (v, cr) = mr(json!({"user": {"can_merge": false}}));
        assert_eq!(edit_info(&v, &cr, "me").caps, EditCapabilities::default());
    }
}
