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
        range_rows: None,
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
    if let Some((s, e)) = pos.range_rows {
        position["line_range"] = json!({
            "start": range_end(new_path, s),
            "end": range_end(new_path, e),
        });
    }
    json!({ "body": body, "position": position })
}

/// GitLab's row id: `sha1(path)_old_new`.
fn line_code(path: &str, r: RowRef) -> String {
    use sha1::{Digest, Sha1};
    let hex: String = Sha1::digest(path.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("{hex}_{}_{}", r.old, r.new)
}

/// One end of `position[line_range]`.
fn range_end(path: &str, r: RowRef) -> Value {
    let mut v = json!({ "line_code": line_code(path, r) });
    match r.kind {
        LineKind::Add => {
            v["type"] = json!("new");
            v["new_line"] = json!(r.new);
        }
        LineKind::Del => {
            v["type"] = json!("old");
            v["old_line"] = json!(r.old);
        }
        LineKind::Context => {
            v["type"] = Value::Null;
            v["old_line"] = json!(r.old);
            v["new_line"] = json!(r.new);
        }
    }
    v
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
    fn range_payload_has_line_codes() {
        let mut p = DiffPosition::new("a.rs", DiffSide::New, 12);
        p.start_line = Some(11);
        let row = |old, new, kind| RowRef { old, new, kind };
        p.range_rows = Some((row(10, 11, LineKind::Context), row(11, 12, LineKind::Add)));
        let r = discussion_payload(&p, ("b", "s", "h"), "x")["position"]["line_range"].clone();
        let code = r["start"]["line_code"].as_str().unwrap().to_string();
        assert!(code.ends_with("_10_11") && code.len() == 40 + 6, "{code}");
        assert_eq!(
            (
                r["start"]["type"].clone(),
                r["start"]["old_line"].as_u64(),
                r["start"]["new_line"].as_u64()
            ),
            (Value::Null, Some(10), Some(11))
        );
        assert_eq!(
            (
                r["end"]["type"].as_str(),
                r["end"].get("old_line"),
                r["end"]["new_line"].as_u64()
            ),
            (Some("new"), None, Some(12))
        );
        assert!(r["end"]["line_code"].as_str().unwrap().ends_with("_11_12"));
        p.range_rows = None;
        assert!(discussion_payload(&p, ("b", "s", "h"), "x")["position"]
            .get("line_range")
            .is_none());
    }

    #[test]
    fn line_code_is_sha1_of_path() {
        let r = RowRef {
            old: 1,
            new: 2,
            kind: LineKind::Context,
        };
        // printf 'README.md' | shasum
        assert_eq!(
            line_code("README.md", r),
            "8ec9a00bfd09b3190ac6b22251dbb1aa95a0579d_1_2"
        );
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
