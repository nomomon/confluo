//! Review submission payloads and timeline events.

use gv_core::*;
use serde_json::{json, Value};

/// Body of `POST /repos/{o}/{r}/pulls/{n}/reviews`; None for actions GitHub has no review for.
pub(crate) fn review_payload(action: ReviewAction, body: Option<&str>) -> Option<Value> {
    let event = match action {
        ReviewAction::Approve => "APPROVE",
        ReviewAction::RequestChanges => "REQUEST_CHANGES",
        ReviewAction::Comment => "COMMENT",
        ReviewAction::Unapprove => return None,
    };
    let mut v = json!({ "event": event });
    if let Some(b) = body.filter(|b| !b.is_empty()) {
        v["body"] = json!(b);
    }
    Some(v)
}

fn review_state(s: &str) -> Option<ReviewState> {
    match s {
        "APPROVED" => Some(ReviewState::Approved),
        "CHANGES_REQUESTED" => Some(ReviewState::ChangesRequested),
        "COMMENTED" => Some(ReviewState::Commented),
        _ => None,
    }
}

/// Opened, verdict reviews and merged/closed from a pull request node.
pub(crate) fn timeline(pr: &Value, summary: &ChangeRequest) -> Vec<TimelineEvent> {
    let mut out = vec![TimelineEvent { kind: TimelineKind::Opened, actor: summary.author.clone(), at: summary.created_at }];
    for r in pr["reviews"]["nodes"].as_array().into_iter().flatten() {
        // Plain comment reviews show as comments; only verdicts become events.
        if let Some(st) = r["state"].as_str().and_then(review_state).filter(|s| *s != ReviewState::Commented) {
            out.push(TimelineEvent { kind: TimelineKind::Reviewed(st), actor: super::user(&r["author"]), at: super::time(&r["createdAt"]) });
        }
    }
    if let Some(at) = pr["mergedAt"].as_str() {
        out.push(TimelineEvent { kind: TimelineKind::Merged, actor: super::user(&pr["mergedBy"]), at: super::time(&json!(at)) });
    } else if summary.state == ChangeRequestState::Closed {
        if let Some(at) = pr["closedAt"].as_str() {
            out.push(TimelineEvent { kind: TimelineKind::Closed, actor: User::default(), at: super::time(&json!(at)) });
        }
    }
    out.sort_by_key(|e| e.at);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads() {
        assert_eq!(review_payload(ReviewAction::Approve, None), Some(json!({"event": "APPROVE"})));
        assert_eq!(review_payload(ReviewAction::RequestChanges, Some("fix")), Some(json!({"event": "REQUEST_CHANGES", "body": "fix"})));
        assert_eq!(review_payload(ReviewAction::Comment, Some("")), Some(json!({"event": "COMMENT"})));
        assert_eq!(review_payload(ReviewAction::Unapprove, None), None);
    }
}
