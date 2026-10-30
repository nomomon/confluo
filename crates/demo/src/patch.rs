//! Helpers that turn readable fixture text into unified diff hunks.

/// Patch of a new file: every line added.
pub fn added(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut out = format!("@@ -0,0 +1,{} @@\n", lines.len());
    for l in lines {
        out.push('+');
        out.push_str(l);
        out.push('\n');
    }
    out
}

/// One hunk from lines prefixed with ' ', '+' or '-'; the header counts are computed.
pub fn hunk(old_start: u32, new_start: u32, body: &str) -> String {
    let (mut old, mut new) = (0, 0);
    for l in body.lines() {
        match l.chars().next() {
            Some('+') => new += 1,
            Some('-') => old += 1,
            _ => {
                old += 1;
                new += 1;
            }
        }
    }
    let mut out = format!("@@ -{old_start},{old} +{new_start},{new} @@\n");
    for l in body.lines() {
        // A blank fixture line is a blank context line.
        out.push_str(if l.is_empty() { " " } else { l });
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_core::parse_unified;

    #[test]
    fn counts_lines() {
        let h = hunk(10, 12, " a\n-b\n+c\n+d\n\n e");
        assert!(h.starts_with("@@ -10,4 +12,5 @@"));
        let parsed = parse_unified(&h);
        assert_eq!(parsed[0].lines.len(), 6);
        assert_eq!(parse_unified(&added("x\ny"))[0].new_lines, 2);
    }
}
