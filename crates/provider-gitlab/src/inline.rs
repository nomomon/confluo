//! Positioned discussions: parsing note positions and building the new-discussion payload.

use gv_core::*;
use serde_json::{json, Value};

fn num(v: &Value) -> Option<u32> {
    v.as_u64().map(|x| x as u32)
}

/// A note's `position` into a DiffPosition; None for non-diff notes.
pub(crate) fn parse_position(p: &Value) -> Option<DiffPosition> {
    if p.is_null() || p["position_type"].as_str().is_some_and(|t| t != "text") {
        return None;
    }
    let new_path = p["new_path"].as_str();
    let old_path = p["old_path"].as_str();
    let (side, line, other) = match (num(&p["new_line"]), num(&p["old_line"])) {
        (Some(n), o) => (DiffSide::New, n, o),
        (None, Some(o)) => (DiffSide::Old, o, None),
        (None, None) => return None,
    };
    let path = match side {
        DiffSide::New => new_path.or(old_path)?,
        DiffSide::Old => old_path.or(new_path)?,
    };
    let start = &p["line_range"]["start"];
    let start_line = match side {
        DiffSide::New => num(&start["new_line"]),
        DiffSide::Old => num(&start["old_line"]),
    }
    .filter(|s| *s < line);
    Some(DiffPosition {
        path: path.to_string(),
        old_path: old_path
            .filter(|o| Some(*o) != new_path)
            .map(str::to_string),
        side,
        line,
        start_line,
        other_line: other,
        head_sha: p["head_sha"].as_str().map(str::to_string),
        base_sha: p["base_sha"].as_str().map(str::to_string),
        start_sha: p["start_sha"].as_str().map(str::to_string),
    })
}

/// Body of `POST /projects/:id/merge_requests/:iid/discussions` for a single-line text note.
/// Context lines carry both `old_line` and `new_line` (`other_line` holds the opposite side).
pub(crate) fn discussion_payload(
    pos: &DiffPosition,
    refs: (&str, &str, &str),
    body: &str,
) -> Value {
    let (base, start, head) = refs;
    let new_path = &pos.path;
    let old_path = pos.old_path.as_deref().unwrap_or(&pos.path);
    let mut position = json!({
        "position_type": "text",
        "base_sha": base,
        "start_sha": start,
        "head_sha": head,
        "new_path": new_path,
        "old_path": old_path,
    });
    match pos.side {
        DiffSide::New => {
            position["new_line"] = json!(pos.line);
            if let Some(o) = pos.other_line {
                position["old_line"] = json!(o);
            }
        }
        DiffSide::Old => {
            position["old_line"] = json!(pos.line);
            if let Some(n) = pos.other_line {
                position["new_line"] = json!(n);
            }
        }
    }
    json!({ "body": body, "position": position })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_positions() {
        let p = parse_position(
            &json!({"position_type": "text", "new_path": "b.rs", "old_path": "a.rs",
            "new_line": 5, "old_line": 4, "head_sha": "h",
            "line_range": {"start": {"new_line": 3}, "end": {"new_line": 5}}}),
        )
        .unwrap();
        assert_eq!(
            (p.path.as_str(), p.side, p.line, p.other_line, p.start_line),
            ("b.rs", DiffSide::New, 5, Some(4), Some(3))
        );
        assert_eq!(p.old_path.as_deref(), Some("a.rs"));
        let o = parse_position(
            &json!({"position_type": "text", "new_path": "a", "old_path": "a", "old_line": 9}),
        )
        .unwrap();
        assert_eq!((o.side, o.line, o.old_path), (DiffSide::Old, 9, None));
        assert!(parse_position(&json!({"position_type": "image", "new_path": "x.png"})).is_none());
        assert!(parse_position(&Value::Null).is_none());
    }

    #[test]
    fn builds_payload() {
        let mut p = DiffPosition::new("x.rs", DiffSide::New, 7);
        let v = discussion_payload(&p, ("b", "s", "h"), "hi");
        let pos = &v["position"];
        assert_eq!(
            (
                pos["position_type"].as_str(),
                pos["base_sha"].as_str(),
                pos["start_sha"].as_str(),
                pos["head_sha"].as_str()
            ),
            (Some("text"), Some("b"), Some("s"), Some("h"))
        );
        assert_eq!(
            (
                pos["new_line"].as_u64(),
                pos.get("old_line"),
                pos["old_path"].as_str()
            ),
            (Some(7), None, Some("x.rs"))
        );
        p.other_line = Some(6);
        let pos = discussion_payload(&p, ("b", "s", "h"), "hi")["position"].clone();
        assert_eq!(
            (pos["new_line"].as_u64(), pos["old_line"].as_u64()),
            (Some(7), Some(6))
        );
        let mut d = DiffPosition::new("old.rs", DiffSide::Old, 3);
        d.old_path = Some("old.rs".into());
        let pos = discussion_payload(&d, ("b", "s", "h"), "x")["position"].clone();
        assert_eq!(
            (pos["old_line"].as_u64(), pos.get("new_line")),
            (Some(3), None)
        );
        assert_eq!(v["body"].as_str(), Some("hi"));
    }
}
