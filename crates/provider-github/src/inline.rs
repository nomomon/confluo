//! Inline review threads: parsing `reviewThreads` nodes and building the REST payload.

use crate::{comment, s};
use gv_core::*;
use serde_json::{json, Value};

/// GraphQL selection for one review thread node.
pub(crate) const THREAD_FIELDS: &str = r#"id isResolved isOutdated path line originalLine startLine originalStartLine diffSide startDiffSide
  comments(first: 50) { nodes { id body createdAt path line originalLine author { login avatarUrl ... on User { name } }
    reactionGroups { content viewerHasReacted reactors(first: 5) { totalCount nodes { ... on User { login } ... on Bot { login } } } } } }"#;

fn side(v: &Value) -> DiffSide {
    if v.as_str() == Some("LEFT") { DiffSide::Old } else { DiffSide::New }
}

fn side_str(s: DiffSide) -> &'static str {
    match s {
        DiffSide::Old => "LEFT",
        DiffSide::New => "RIGHT",
    }
}

fn num(v: &Value) -> Option<u32> {
    v.as_u64().map(|x| x as u32)
}

/// One `reviewThreads` node into a Thread with its full anchor.
pub(crate) fn parse_thread(t: &Value) -> Thread {
    let outdated = t["isOutdated"] == true || t["line"].is_null();
    let line = num(&t["line"]).or_else(|| num(&t["originalLine"])).unwrap_or(0);
    let start = num(&t["startLine"]).or_else(|| num(&t["originalStartLine"])).filter(|s| *s != line);
    let position = t["path"].as_str().map(|p| DiffPosition {
        path: p.to_string(),
        side: side(&t["diffSide"]),
        line,
        start_line: start,
        ..Default::default()
    });
    Thread {
        id: s(&t["id"]),
        resolved: t["isResolved"].as_bool(),
        comments: t["comments"]["nodes"].as_array().into_iter().flatten().map(comment).collect(),
        position,
        outdated,
    }
}

/// Body of `POST /repos/{o}/{r}/pulls/{n}/comments`.
pub(crate) fn line_comment_payload(pos: &DiffPosition, head_sha: &str, body: &str) -> Value {
    let mut v = json!({
        "body": body,
        "commit_id": head_sha,
        "path": pos.path,
        "line": pos.line,
        "side": side_str(pos.side),
    });
    if let Some(start) = pos.start_line.filter(|s| *s < pos.line) {
        v["start_line"] = json!(start);
        v["start_side"] = json!(side_str(pos.side));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_thread() {
        let t = parse_thread(&json!({"id": "PRRT_1", "isResolved": true, "isOutdated": false, "path": "a.rs",
            "line": 9, "startLine": 7, "diffSide": "LEFT", "comments": {"nodes": [{"id": "c", "body": "x"}]}}));
        let p = t.position.unwrap();
        assert_eq!((p.path.as_str(), p.side, p.line, p.start_line), ("a.rs", DiffSide::Old, 9, Some(7)));
        assert_eq!((t.resolved, t.outdated, t.comments.len()), (Some(true), false, 1));
        let o = parse_thread(&json!({"id": "x", "path": "b", "line": null, "originalLine": 3, "isOutdated": true}));
        assert!(o.outdated);
        assert_eq!(o.position.unwrap().line, 3);
    }

    #[test]
    fn builds_payload() {
        let mut p = DiffPosition::new("src/x.rs", DiffSide::New, 12);
        let v = line_comment_payload(&p, "abc", "hi");
        assert_eq!((v["side"].as_str(), v["line"].as_u64(), v.get("start_line")), (Some("RIGHT"), Some(12), None));
        p.start_line = Some(10);
        p.side = DiffSide::Old;
        let v = line_comment_payload(&p, "abc", "hi");
        assert_eq!((v["start_line"].as_u64(), v["start_side"].as_str(), v["commit_id"].as_str()), (Some(10), Some("LEFT"), Some("abc")));
    }
}
