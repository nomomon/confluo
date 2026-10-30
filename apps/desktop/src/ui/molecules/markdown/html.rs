//! Raw HTML cleanup before parsing: drop comments, turn `<details>` into fence markers the parser
//! folds into `Block::Details`, and strip a few inline tags.

/// Paragraph that opens a details block; the rest of the line is the summary.
pub const DETAILS_OPEN: &str = "\u{2063}details:";
/// Paragraph that closes it.
pub const DETAILS_CLOSE: &str = "\u{2063}/details";

pub fn preprocess(src: &str) -> String {
    mark_details(&strip_comments(src))
}

fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = match rest[start..].find("-->") {
            Some(end) => &rest[start + end + 3..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// `<details><summary>S</summary>` becomes an open marker paragraph, `</details>` a close one.
fn mark_details(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    loop {
        let open = rest.find("<details");
        let close = rest.find("</details>");
        match (open, close) {
            (Some(o), c) if c.is_none_or(|c| o < c) => {
                out.push_str(&rest[..o]);
                let after = &rest[o..];
                let tag_end = after.find('>').map(|e| e + 1).unwrap_or(after.len());
                let mut body = &after[tag_end..];
                let mut summary = "Details".to_string();
                let trimmed = body.trim_start();
                if trimmed.starts_with("<summary") {
                    if let Some(e) = trimmed.find("</summary>") {
                        let inner = &trimmed[trimmed.find('>').map(|i| i + 1).unwrap_or(0)..e];
                        let s = strip_tags(inner).trim().to_string();
                        if !s.is_empty() {
                            summary = s;
                        }
                        body = &trimmed[e + "</summary>".len()..];
                    }
                }
                out.push_str(&format!("\n\n{DETAILS_OPEN}{summary}\n\n"));
                rest = body;
            }
            (_, Some(c)) => {
                out.push_str(&rest[..c]);
                out.push_str(&format!("\n\n{DETAILS_CLOSE}\n\n"));
                rest = &rest[c + "</details>".len()..];
            }
            _ => break,
        }
    }
    out.push_str(rest);
    out
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_comments_and_marks_details() {
        let src = "a<!-- hidden -->b\n<details><summary><b>Log</b></summary>\n\nlots\n</details>\nc";
        let out = preprocess(src);
        assert!(out.starts_with("ab\n"));
        assert!(out.contains(&format!("{DETAILS_OPEN}Log")));
        assert!(out.contains("lots"));
        assert!(out.contains(DETAILS_CLOSE));
        assert!(out.ends_with("\nc"));
    }

    #[test]
    fn nested_details_and_missing_summary() {
        let out = preprocess("<details>\n<details><summary>In</summary>x</details></details>");
        assert_eq!(out.matches(DETAILS_OPEN).count(), 2);
        assert_eq!(out.matches(DETAILS_CLOSE).count(), 2);
        assert!(out.contains(&format!("{DETAILS_OPEN}Details")));
    }
}
