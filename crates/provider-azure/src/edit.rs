//! Write requests (comments, votes, edits, completion) and which of them a PR allows.

use gv_core::*;
use serde_json::{json, Value};

use crate::checks;

/// Thread status values of the 7.1 API (`CommentThreadStatus`).
const ACTIVE: &str = "active";
const FIXED: &str = "fixed";

/// `POST .../threads`: a new thread with one comment, anchored to a diff line when `pos` is set.
pub(crate) fn thread_payload(body: &str, pos: Option<&DiffPosition>) -> Value {
    let mut v = json!({
        "comments": [{ "parentCommentId": 0, "content": body, "commentType": "text" }],
        "status": ACTIVE,
    });
    if let Some(p) = pos {
        let start = p.start_line.filter(|s| *s < p.line).unwrap_or(p.line);
        let (from, to) = match p.side {
            DiffSide::New => ("rightFileStart", "rightFileEnd"),
            DiffSide::Old => ("leftFileStart", "leftFileEnd"),
        };
        let path = match p.side {
            DiffSide::Old => p.old_path.as_deref().unwrap_or(&p.path),
            DiffSide::New => &p.path,
        };
        v["threadContext"] = json!({
            "filePath": format!("/{}", path.trim_start_matches('/')),
            from: { "line": start, "offset": 1 },
            to: { "line": p.line, "offset": 1 },
        });
    }
    v
}

/// `POST .../threads/{id}/comments`: a reply under the thread's first comment.
pub(crate) fn reply_payload(body: &str) -> Value {
    json!({ "parentCommentId": 1, "content": body, "commentType": "text" })
}

/// `PATCH .../threads/{id}`.
pub(crate) fn resolve_payload(resolved: bool) -> Value {
    json!({ "status": if resolved { FIXED } else { ACTIVE } })
}

/// `PUT .../reviewers/{id}`.
pub(crate) fn vote_payload(vote: i64) -> Value {
    json!({ "vote": vote })
}

/// `PATCH .../pullRequests/{id}` with a new title and/or description.
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

/// Abandon (`open == false`) or reactivate.
pub(crate) fn status_payload(open: bool) -> Value {
    json!({ "status": if open { "active" } else { "abandoned" } })
}

pub(crate) fn merge_strategy(o: &MergeOptions) -> &'static str {
    match o.method {
        MergeMethod::Squash => "squash",
        MergeMethod::Merge if o.squash => "squash",
        MergeMethod::Merge => "noFastForward",
        MergeMethod::Rebase => "rebase",
        MergeMethod::RebaseMerge => "rebaseMerge",
    }
}

/// Complete now (needs the current source commit) or set auto-complete (`when_pipeline_succeeds`).
pub(crate) fn complete_payload(o: &MergeOptions, head: &str, my_id: &str) -> Value {
    let options =
        json!({ "mergeStrategy": merge_strategy(o), "deleteSourceBranch": o.delete_branch });
    if o.when_pipeline_succeeds {
        json!({ "autoCompleteSetBy": { "id": my_id }, "completionOptions": options })
    } else {
        json!({ "status": "completed", "lastMergeSourceCommit": { "commitId": head }, "completionOptions": options })
    }
}

/// Reviewer ids to add and remove going from `current` to `desired`.
pub(crate) fn reviewer_changes(
    current: &[User],
    desired: &[User],
) -> ProviderResult<(Vec<String>, Vec<String>)> {
    let id = |u: &User| {
        u.id.clone()
            .ok_or_else(|| ProviderError::Decode(format!("no Azure DevOps id for {}", u.username)))
    };
    let has = |list: &[User], u: &User| {
        list.iter()
            .any(|x| x.id == u.id || x.username.eq_ignore_ascii_case(&u.username))
    };
    let add = desired
        .iter()
        .filter(|u| !has(current, u))
        .map(id)
        .collect::<ProviderResult<Vec<_>>>()?;
    let remove = current
        .iter()
        .filter(|u| !has(desired, u))
        .map(id)
        .collect::<ProviderResult<Vec<_>>>()?;
    Ok((add, remove))
}

/// Edit info from the single-PR payload and its policy evaluations. Azure checks permissions on
/// write, so contributors get every action and a refused one shows the server's message.
pub(crate) fn edit_info(v: &Value, cr: &ChangeRequest, evals: &[Value]) -> EditInfo {
    let open = cr.state.is_active();
    let mut caps = EditCapabilities::editor(open);
    caps.assignees = false;
    caps.reopen = cr.state == ChangeRequestState::Closed;
    if open {
        let (failed, running) = checks::blocking(evals);
        let blocked = if cr.state == ChangeRequestState::Draft {
            Some("Draft pull requests can't be completed".to_string())
        } else {
            match v["mergeStatus"].as_str().unwrap_or_default() {
                "conflicts" => Some("This branch has conflicts".into()),
                "rejectedByPolicy" => Some("A branch policy rejects the merge".into()),
                "failure" => Some("The test merge failed".into()),
                "queued" => Some("Azure DevOps is still checking mergeability".into()),
                _ if failed => Some("Required policies are not met".into()),
                _ => None,
            }
        };
        match blocked {
            Some(why) => caps.merge_blocked = Some(why),
            None => {
                caps.merge = true;
                let preferred = match v["completionOptions"]["mergeStrategy"].as_str() {
                    Some("squash") => MergeMethod::Squash,
                    Some("rebase") => MergeMethod::Rebase,
                    Some("rebaseMerge") => MergeMethod::RebaseMerge,
                    _ => MergeMethod::Merge,
                };
                let mut methods = vec![preferred];
                methods.extend(
                    [
                        MergeMethod::Merge,
                        MergeMethod::Squash,
                        MergeMethod::Rebase,
                        MergeMethod::RebaseMerge,
                    ]
                    .into_iter()
                    .filter(|m| *m != preferred),
                );
                caps.merge_methods = methods;
                caps.delete_branch_option = true;
                caps.delete_branch_default = v["completionOptions"]["deleteSourceBranch"] == true;
                caps.auto_merge = running;
            }
        }
    }
    EditInfo {
        caps,
        assignees: vec![],
        node_id: None,
    }
}

/// `DELETE .../pullRequests/{id}/threads/{t}/comments/{c}` for comment id `t/c`.
pub(crate) fn delete_comment_url(pr_url: &str, comment_id: &str) -> ProviderResult<String> {
    let (t, c) = crate::parse::split_comment_id(comment_id)?;
    Ok(format!("{pr_url}/threads/{t}/comments/{c}?{}", crate::API))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_pr;

    #[test]
    fn delete_comment_url_names_thread_and_comment() {
        assert_eq!(delete_comment_url("https://dev.azure.com/o/p/_apis/git/repositories/r/pullRequests/5", "12/3").unwrap(), "https://dev.azure.com/o/p/_apis/git/repositories/r/pullRequests/5/threads/12/comments/3?api-version=7.1");
        assert!(delete_comment_url("x", "nope").is_err());
    }

    fn pr(extra: Value) -> (Value, ChangeRequest) {
        let mut v: Value =
            serde_json::from_str(include_str!("../fixtures/pull_request.json")).unwrap();
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let cr = parse_pr(
            "dev.azure.com",
            "contoso",
            &v,
            &User::named("me@example.com"),
        );
        (v, cr)
    }

    #[test]
    fn comment_payloads() {
        assert_eq!(
            thread_payload("hi", None),
            json!({"comments": [{"parentCommentId": 0, "content": "hi", "commentType": "text"}], "status": "active"})
        );
        let mut p = DiffPosition::new("src/app.ts", DiffSide::New, 14);
        p.start_line = Some(12);
        let v = thread_payload("x", Some(&p));
        assert_eq!(
            v["threadContext"],
            json!({"filePath": "/src/app.ts", "rightFileStart": {"line": 12, "offset": 1}, "rightFileEnd": {"line": 14, "offset": 1}})
        );
        let mut o = DiffPosition::new("new.rs", DiffSide::Old, 3);
        o.old_path = Some("old.rs".into());
        let v = thread_payload("x", Some(&o));
        assert_eq!(
            v["threadContext"],
            json!({"filePath": "/old.rs", "leftFileStart": {"line": 3, "offset": 1}, "leftFileEnd": {"line": 3, "offset": 1}})
        );
        assert_eq!(
            reply_payload("ok"),
            json!({"parentCommentId": 1, "content": "ok", "commentType": "text"})
        );
        assert_eq!(resolve_payload(true), json!({"status": "fixed"}));
        assert_eq!(resolve_payload(false), json!({"status": "active"}));
    }

    #[test]
    fn pr_payloads() {
        assert_eq!(vote_payload(-5), json!({"vote": -5}));
        assert_eq!(update_payload(Some("T"), None), json!({"title": "T"}));
        assert_eq!(update_payload(None, Some("B")), json!({"description": "B"}));
        assert_eq!(status_payload(false), json!({"status": "abandoned"}));
        assert_eq!(status_payload(true), json!({"status": "active"}));
        let o = MergeOptions {
            method: MergeMethod::Squash,
            delete_branch: true,
            ..Default::default()
        };
        assert_eq!(
            complete_payload(&o, "abc", "me"),
            json!({"status": "completed", "lastMergeSourceCommit": {"commitId": "abc"}, "completionOptions": {"mergeStrategy": "squash", "deleteSourceBranch": true}})
        );
        let auto = MergeOptions {
            method: MergeMethod::RebaseMerge,
            when_pipeline_succeeds: true,
            ..Default::default()
        };
        assert_eq!(
            complete_payload(&auto, "abc", "me-id"),
            json!({"autoCompleteSetBy": {"id": "me-id"}, "completionOptions": {"mergeStrategy": "rebaseMerge", "deleteSourceBranch": false}})
        );
        assert_eq!(merge_strategy(&MergeOptions::default()), "noFastForward");
        assert_eq!(
            merge_strategy(&MergeOptions {
                method: MergeMethod::Rebase,
                ..Default::default()
            }),
            "rebase"
        );
        assert_eq!(
            merge_strategy(&MergeOptions {
                squash: true,
                ..Default::default()
            }),
            "squash"
        );
    }

    #[test]
    fn reviewers_by_id() {
        let u = |n: &str, id: Option<&str>| User {
            id: id.map(str::to_string),
            ..User::named(n)
        };
        let (add, remove) = reviewer_changes(
            &[u("a", Some("1")), u("b", Some("2"))],
            &[u("a", Some("1")), u("c", Some("3"))],
        )
        .unwrap();
        assert_eq!(
            (add, remove),
            (vec!["3".to_string()], vec!["2".to_string()])
        );
        assert!(reviewer_changes(&[], &[u("x", None)]).is_err());
    }

    #[test]
    fn capabilities() {
        let evals: Value =
            serde_json::from_str(include_str!("../fixtures/policy_evaluations.json")).unwrap();
        let evals = evals["value"].as_array().unwrap().clone();
        let (v, cr) = pr(json!({}));
        let e = edit_info(&v, &cr, &[]);
        assert!(
            e.caps.title
                && e.caps.draft
                && e.caps.close
                && !e.caps.reopen
                && !e.caps.assignees
                && e.caps.merge
        );
        assert_eq!(e.caps.merge_methods[0], MergeMethod::Squash);
        assert_eq!(e.caps.merge_methods.len(), 4);
        assert!(e.caps.delete_branch_default && !e.caps.auto_merge);
        assert_eq!(
            edit_info(&v, &cr, &evals).caps.merge_blocked.as_deref(),
            Some("Required policies are not met")
        );
        let running: Vec<Value> = evals
            .iter()
            .filter(|e| e["status"] != "rejected")
            .cloned()
            .collect();
        assert!(edit_info(&v, &cr, &running).caps.auto_merge);
        let (v, cr) = pr(json!({"mergeStatus": "conflicts"}));
        assert_eq!(
            edit_info(&v, &cr, &[]).caps.merge_blocked.as_deref(),
            Some("This branch has conflicts")
        );
        let (v, cr) = pr(json!({"isDraft": true}));
        assert!(!edit_info(&v, &cr, &[]).caps.merge);
        let (v, cr) = pr(json!({"status": "abandoned"}));
        let e = edit_info(&v, &cr, &[]);
        assert!(e.caps.reopen && !e.caps.close && !e.caps.merge);
    }
}
