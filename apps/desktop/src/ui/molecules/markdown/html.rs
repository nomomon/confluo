//! Raw HTML cleanup before parsing: drop comments, collapse `<details>`.

pub fn preprocess(src: &str) -> String {
    collapse_details(&strip_comments(src))
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

/// Replace each `<details>` block with its summary as one muted line.
fn collapse_details(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(start) = rest.find("<details") {
        out.push_str(&rest[..start]);
        let block_end = rest[start..].find("</details>").map(|e| start + e + "</details>".len()).unwrap_or(rest.len());
        let block = &rest[start..block_end];
        let summary = block
            .find("<summary>")
            .and_then(|s| block[s + 9..].find("</summary>").map(|e| strip_tags(&block[s + 9..s + 9 + e])))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Details".into());
        out.push_str(&format!("\n\n*{} (collapsed)*\n\n", summary.trim()));
        rest = &rest[block_end..];
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
    use super::preprocess;

    #[test]
    fn drops_comments_and_collapses_details() {
        let src = "a<!-- hidden -->b\n<details><summary><b>Log</b></summary>\nlots\n</details>\nc";
        let out = preprocess(src);
        assert!(out.starts_with("ab\n"));
        assert!(out.contains("*Log (collapsed)*"));
        assert!(!out.contains("lots"));
        assert!(out.ends_with("\nc"));
    }
}
