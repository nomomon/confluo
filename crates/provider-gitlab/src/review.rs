//! Approvals, review submission and timeline events from system notes.

use gv_core::*;
use serde_json::Value;

/// Method and path suffix (after the MR path) for a review action; None if GitLab has none.
pub(crate) fn review_request(action: ReviewAction) -> Option<(reqwest::Method, &'static str)> {
    match action {
        ReviewAction::Approve => Some((reqwest::Method::POST, "/approve")),
        ReviewAction::Unapprove => Some((reqwest::Method::POST, "/unapprove")),
        ReviewAction::Comment => Some((reqwest::Method::POST, "/notes")),
        // Reviewer "requested changes" state has no stable public endpoint; not offered.
        ReviewAction::RequestChanges => None,
    }
}

/// Review info from `GET .../approvals`.
pub(crate) fn review_info(cr: &ChangeRequest, me: &str, approvals: &Value) -> ReviewInfo {
    let approved_by_me = approvals["user_has_approved"] == true
        || approvals["approved_by"].as_array().into_iter().flatten().any(|a| a["user"]["username"].as_str().is_some_and(|u| u.eq_ignore_ascii_case(me)));
    let can_approve = approvals["user_can_approve"].as_bool().unwrap_or(!cr.author.username.eq_ignore_ascii_case(me));
    let given = approvals["approved_by"].as_array().map_or(0, |a| a.len() as u32);
    let required = approvals["approvals_required"].as_u64().unwrap_or(0) as u32;
    let mut my_state = my_review_state(cr, me);
    if approved_by_me {
        my_state = Some(ReviewState::Approved);
    } else if my_state == Some(ReviewState::Approved) {
        my_state = None;
    }
    ReviewInfo { caps: gitlab_capabilities(cr, me, approved_by_me, can_approve), my_state, approvals: Some(Approvals { given, required }) }
}

/// Timeline event for a system note, if it is one we show.
pub(crate) fn system_event(note: &Value) -> Option<TimelineEvent> {
    let body = note["body"].as_str()?;
    let kind = if body.starts_with("approved this merge request") {
        TimelineKind::Reviewed(ReviewState::Approved)
    } else if body.starts_with("requested changes") {
        TimelineKind::Reviewed(ReviewState::ChangesRequested)
    } else if body == "merged" || body.starts_with("merged ") {
        TimelineKind::Merged
    } else if body == "closed" {
        TimelineKind::Closed
    } else {
        return None;
    };
    Some(TimelineEvent { kind, actor: super::user(&note["author"]), at: super::time(&note["created_at"]) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn requests_and_events() {
        assert_eq!(review_request(ReviewAction::Approve).unwrap().1, "/approve");
        assert_eq!(review_request(ReviewAction::Unapprove).unwrap().1, "/unapprove");
        assert!(review_request(ReviewAction::RequestChanges).is_none());
        let e = system_event(&json!({"body": "approved this merge request", "author": {"username": "ada"}, "created_at": "2024-01-01T00:00:00Z"})).unwrap();
        assert_eq!(e.kind, TimelineKind::Reviewed(ReviewState::Approved));
        assert!(system_event(&json!({"body": "added 2 commits"})).is_none());
    }

    #[test]
    fn info_from_approvals() {
        let cr = crate::parse_mr("gitlab.com", &json!({"iid": 1, "author": {"username": "ada"}, "state": "opened", "web_url": "https://gitlab.com/g/p/-/merge_requests/1"}));
        let a = json!({"user_has_approved": true, "user_can_approve": false, "approvals_required": 2, "approved_by": [{"user": {"username": "me"}}]});
        let i = review_info(&cr, "me", &a);
        assert_eq!(i.my_state, Some(ReviewState::Approved));
        assert!(i.caps.unapprove && !i.caps.approve);
        assert_eq!(i.approvals, Some(Approvals { given: 1, required: 2 }));
    }
}
