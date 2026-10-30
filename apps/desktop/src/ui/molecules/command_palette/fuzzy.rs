//! Subsequence fuzzy match with a score favoring word starts and runs.

/// Score of `query` against `text` (both compared lowercase), `None` if not a subsequence.
pub fn score(query: &str, text: &str) -> Option<i32> {
    let q: Vec<char> = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if q.is_empty() {
        return Some(0);
    }
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut qi, mut s, mut run, mut prev) = (0, 0i32, 0i32, None::<usize>);
    for (i, c) in t.iter().enumerate() {
        if qi < q.len() && *c == q[qi] {
            let word_start = i == 0 || !t[i - 1].is_alphanumeric();
            run = if prev == Some(i.wrapping_sub(1)) {
                run + 1
            } else {
                0
            };
            s += 1 + run * 3 + if word_start { 6 } else { 0 };
            prev = Some(i);
            qi += 1;
        }
    }
    (qi == q.len()).then(|| s * 100 / (t.len() as i32 + 10))
}

#[cfg(test)]
mod tests {
    use super::score;

    #[test]
    fn matches_subsequences_and_ranks_prefixes() {
        assert!(score("rfr", "Refresh").is_some());
        assert!(score("xyz", "Refresh").is_none());
        assert!(
            score("set", "Settings").unwrap() > score("set", "Toggle sidebar theme").unwrap_or(0)
        );
        assert_eq!(score("", "anything"), Some(0));
    }
}
