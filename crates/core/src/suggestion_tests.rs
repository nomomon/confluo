use super::*;
use crate::FileStatus;

fn sug(above: u32, below: u32, lines: &[&str]) -> BodyPart {
    BodyPart::Suggestion(Suggestion {
        above,
        below,
        lines: lines.iter().map(|s| s.to_string()).collect(),
    })
}

#[test]
fn github_suggestion_splits_text() {
    let body = "Nit: rename.\n\n```suggestion\nlet total = 1;\n```\nThanks";
    assert_eq!(
        split_suggestions(body),
        vec![
            BodyPart::Text("Nit: rename.\n".into()),
            sug(0, 0, &["let total = 1;"]),
            BodyPart::Text("Thanks".into()),
        ]
    );
}

#[test]
fn gitlab_range_form() {
    let body = "````suggestion:-1+2\na\n\nb\n````";
    assert_eq!(split_suggestions(body), vec![sug(1, 2, &["a", "", "b"])]);
}

#[test]
fn empty_suggestion_deletes_lines() {
    assert_eq!(
        split_suggestions("```suggestion\n```"),
        vec![sug(0, 0, &[])]
    );
}

#[test]
fn other_fences_and_unclosed_stay_text() {
    let body = "```rust\nfn a() {}\n```";
    assert_eq!(split_suggestions(body), vec![BodyPart::Text(body.into())]);
    let open = "```suggestion\nx";
    assert_eq!(split_suggestions(open), vec![BodyPart::Text(open.into())]);
    assert!(split_suggestions("```suggestion:-x+1\nx\n```").len() == 1);
    assert!(matches!(
        &split_suggestions("```suggestion:-x+1\nx\n```")[0],
        BodyPart::Text(_)
    ));
}

fn file() -> ChangedFile {
    let patch = "@@ -1,4 +1,5 @@\n a\n b\n-c\n+C\n+D\n d\n";
    ChangedFile::from_patch("f.rs", "f.rs", FileStatus::Modified, patch)
}

#[test]
fn context_ends_at_anchor() {
    let pos = DiffPosition::new("f.rs", DiffSide::New, 4);
    let ctx = code_context(&file(), &pos, 2, 4);
    let texts: Vec<&str> = ctx.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(texts, vec!["c", "C", "D"]);
    let wide = code_context(&file(), &pos, 9, 4);
    assert_eq!(wide.len(), 4);
    let old = DiffPosition::new("f.rs", DiffSide::Old, 3);
    assert_eq!(code_context(&file(), &old, 1, 4).last().unwrap().text, "c");
    let missing = DiffPosition::new("f.rs", DiffSide::New, 40);
    assert!(code_context(&file(), &missing, 2, 4).is_empty());
}

#[test]
fn original_lines_follow_offsets() {
    let pos = DiffPosition::new("f.rs", DiffSide::New, 3);
    let s = Suggestion {
        above: 1,
        below: 1,
        lines: vec![],
    };
    assert_eq!(
        suggestion_original(&file(), &pos, &s),
        Some((2, vec!["b".into(), "C".into(), "D".into()]))
    );
    let far = Suggestion {
        above: 9,
        below: 0,
        lines: vec![],
    };
    assert_eq!(suggestion_original(&file(), &pos, &far), None);
}
