//! Command palette ranking: prefixes (`>` actions, `#` PRs, `@` files, `/` views, `?` shortcuts),
//! fuzzy scores, and a most-recently-used boost. Pure, so it is unit tested.

use crate::ui::molecules::command_palette::score;

/// Most recently run commands kept (persisted in settings.json).
pub const MRU_MAX: usize = 10;

/// Palette groups in their empty-query order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Group {
    Suggested,
    GoTo,
    Recent,
    Actions,
    Prs,
    Files,
}

impl Group {
    pub fn title(self) -> &'static str {
        match self {
            Group::Suggested => "Suggested",
            Group::GoTo => "Go to",
            Group::Recent => "Recent pull requests",
            Group::Actions => "Actions",
            Group::Prs => "Pull requests",
            Group::Files => "Files",
        }
    }
}

/// One candidate row.
#[derive(Clone, Debug)]
pub struct Cand {
    /// Stable id for the MRU list, e.g. `a:Approve`, `go:inbox`, `pr:<url>`.
    pub key: String,
    pub group: Group,
    /// Text the fuzzy match runs on.
    pub text: String,
    /// PR number, for `#123`.
    pub number: Option<u64>,
    pub has_shortcut: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prefix {
    None,
    Actions,
    Prs,
    Files,
    Views,
    Help,
}

/// Split a leading prefix character off the query.
pub fn split_prefix(q: &str) -> (Prefix, &str) {
    let q = q.trim_start();
    let p = match q.chars().next() {
        Some('>') => Prefix::Actions,
        Some('#') => Prefix::Prs,
        Some('@') => Prefix::Files,
        Some('/') => Prefix::Views,
        Some('?') => Prefix::Help,
        _ => return (Prefix::None, q.trim()),
    };
    (p, q[1..].trim())
}

/// Key of the cheat sheet command, listed first for `?`.
pub const SHORTCUTS_KEY: &str = "a:ShowShortcuts";

fn allowed(p: Prefix, empty: bool, c: &Cand) -> bool {
    let g = c.group;
    match p {
        Prefix::None if empty => matches!(g, Group::Suggested | Group::GoTo | Group::Recent | Group::Actions),
        Prefix::None => g != Group::Recent,
        Prefix::Actions => matches!(g, Group::Suggested | Group::Actions),
        Prefix::Prs => g == Group::Prs,
        Prefix::Files => g == Group::Files,
        Prefix::Views => g == Group::GoTo,
        Prefix::Help => matches!(g, Group::Suggested | Group::Actions) && (c.has_shortcut || c.key == SHORTCUTS_KEY),
    }
}

fn mru_bonus(mru: &[String], key: &str) -> i32 {
    mru.iter().position(|k| k == key).map(|i| (MRU_MAX - i.min(MRU_MAX - 1)) as i32 * 8).unwrap_or(0)
}

/// Indices of `cands` to show, in order. Groups stay contiguous (one heading each).
pub fn rank(cands: &[Cand], query: &str, mru: &[String]) -> Vec<usize> {
    let (prefix, rest) = split_prefix(query);
    let empty = rest.is_empty();
    let digits = !empty && rest.chars().all(|c| c.is_ascii_digit());
    let mut hits: Vec<(Group, i32, usize)> = cands
        .iter()
        .enumerate()
        .filter(|(_, c)| allowed(prefix, empty, c))
        .filter_map(|(i, c)| {
            let bonus = mru_bonus(mru, &c.key);
            if empty {
                return Some((c.group, bonus, i));
            }
            let num = c.number.map(|n| n.to_string());
            let s = match (&num, digits) {
                (Some(n), true) if *n == rest => Some(100_000),
                (Some(n), true) if prefix == Prefix::Prs && n.starts_with(rest) => Some(50_000 - n.len() as i32),
                (_, true) if prefix == Prefix::Prs => None,
                _ => score(rest, &c.text),
            };
            s.map(|s| (c.group, s + bonus, i))
        })
        .collect();
    // Best score per group decides the group order; with no query, the fixed group order.
    let mut best: Vec<(Group, i32)> = vec![];
    for (g, s, _) in &hits {
        match best.iter_mut().find(|(bg, _)| bg == g) {
            Some(b) => b.1 = b.1.max(*s),
            None => best.push((*g, *s)),
        }
    }
    let gorder = |g: Group| -> (i32, Group) {
        let b = best.iter().find(|(bg, _)| *bg == g).map(|b| b.1).unwrap_or(0);
        if empty {
            (0, g)
        } else {
            (-b, g)
        }
    };
    hits.sort_by(|a, b| gorder(a.0).cmp(&gorder(b.0)).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
    let mut out: Vec<usize> = hits.into_iter().map(|(_, _, i)| i).collect();
    // `?` leads with the cheat sheet itself.
    if prefix == Prefix::Help {
        if let Some(p) = out.iter().position(|i| cands[*i].key == SHORTCUTS_KEY) {
            let i = out.remove(p);
            out.insert(0, i);
        }
    }
    out
}

/// Move `key` to the front of the MRU list, capped at `MRU_MAX`.
pub fn push_mru(mru: &mut Vec<String>, key: &str) {
    mru.retain(|k| k != key);
    mru.insert(0, key.to_string());
    mru.truncate(MRU_MAX);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(key: &str, group: Group, text: &str, number: Option<u64>) -> Cand {
        Cand { key: key.into(), group, text: text.into(), number, has_shortcut: key.starts_with("a:") }
    }

    fn set() -> Vec<Cand> {
        vec![
            c("a:Approve", Group::Suggested, "Approve", None),
            c("go:inbox", Group::GoTo, "Go to Inbox", None),
            c("pr:recent", Group::Recent, "Fix login", Some(7)),
            c("a:Refresh", Group::Actions, "Refresh", None),
            c("a:ToggleTheme", Group::Actions, "Toggle light / dark", None),
            c("pr:5", Group::Prs, "Add search 5", Some(5)),
            c("pr:51", Group::Prs, "Refactor list", Some(51)),
            c("pr:15", Group::Prs, "Bump deps", Some(15)),
            c("f:src/main.rs", Group::Files, "src/main.rs", None),
            c("a:ShowShortcuts", Group::Actions, "Keyboard shortcuts", None),
        ]
    }

    fn keys(q: &str, mru: &[String]) -> Vec<String> {
        let s = set();
        rank(&s, q, mru).into_iter().map(|i| s[i].key.clone()).collect()
    }

    #[test]
    fn prefixes_split() {
        assert_eq!(split_prefix(" > ref"), (Prefix::Actions, "ref"));
        assert_eq!(split_prefix("#12"), (Prefix::Prs, "12"));
        assert_eq!(split_prefix("abc "), (Prefix::None, "abc"));
    }

    #[test]
    fn empty_query_shows_groups_in_order_without_all_prs_or_files() {
        assert_eq!(keys("", &[]), ["a:Approve", "go:inbox", "pr:recent", "a:Refresh", "a:ToggleTheme", "a:ShowShortcuts"]);
    }

    #[test]
    fn number_prefix_matches_exact_then_prefixes() {
        assert_eq!(keys("#5", &[]), ["pr:5", "pr:51"]);
        assert_eq!(keys("#", &[]), ["pr:5", "pr:51", "pr:15"]);
    }

    #[test]
    fn other_prefixes_filter_groups() {
        assert_eq!(keys(">", &[]), ["a:Approve", "a:Refresh", "a:ToggleTheme", "a:ShowShortcuts"]);
        assert_eq!(keys("@main", &[]), ["f:src/main.rs"]);
        assert_eq!(keys("/", &[]), ["go:inbox"]);
        assert_eq!(keys("?", &[])[0], "a:ShowShortcuts");
        assert!(!keys("> re", &[]).iter().any(|k| k.starts_with("pr:")));
    }

    #[test]
    fn plain_query_searches_everything_but_recent() {
        let k = keys("re", &[]);
        assert!(k.contains(&"a:Refresh".to_string()) && k.contains(&"pr:51".to_string()));
        assert!(!k.contains(&"pr:recent".to_string()));
    }

    #[test]
    fn mru_ranks_first_within_group() {
        assert_eq!(keys(">", &["a:ToggleTheme".into()])[1], "a:ToggleTheme");
        let t = keys("t", &["a:ToggleTheme".into()]);
        let refresh = t.iter().position(|k| k == "a:Refresh");
        let theme = t.iter().position(|k| k == "a:ToggleTheme").unwrap();
        assert!(refresh.is_none_or(|r| theme < r));
    }

    #[test]
    fn mru_list_dedupes_and_caps() {
        let mut m = vec![];
        for i in 0..12 {
            push_mru(&mut m, &format!("k{i}"));
        }
        push_mru(&mut m, "k5");
        assert_eq!(m.len(), MRU_MAX);
        assert_eq!(m[0], "k5");
        assert_eq!(m.iter().filter(|k| *k == "k5").count(), 1);
    }
}
