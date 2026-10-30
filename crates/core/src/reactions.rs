//! Reactions (GitHub) and award emoji (GitLab), commits of a PR and per-file viewed state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The eight reactions both providers share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReactionKind {
    ThumbsUp,
    ThumbsDown,
    Laugh,
    Hooray,
    Confused,
    Heart,
    Rocket,
    Eyes,
}

impl ReactionKind {
    pub const ALL: [ReactionKind; 8] = [
        Self::ThumbsUp,
        Self::ThumbsDown,
        Self::Laugh,
        Self::Hooray,
        Self::Confused,
        Self::Heart,
        Self::Rocket,
        Self::Eyes,
    ];

    /// Reactions a provider supports (Azure DevOps comments only have a like).
    pub fn for_provider(kind: crate::ProviderKind) -> &'static [ReactionKind] {
        match kind {
            crate::ProviderKind::Azure => &[Self::ThumbsUp],
            _ => &Self::ALL,
        }
    }

    /// GitHub REST `content` value.
    pub fn github_rest(self) -> &'static str {
        match self {
            Self::ThumbsUp => "+1",
            Self::ThumbsDown => "-1",
            Self::Laugh => "laugh",
            Self::Hooray => "hooray",
            Self::Confused => "confused",
            Self::Heart => "heart",
            Self::Rocket => "rocket",
            Self::Eyes => "eyes",
        }
    }

    /// GitHub GraphQL `ReactionContent` enum value.
    pub fn github_graphql(self) -> &'static str {
        match self {
            Self::ThumbsUp => "THUMBS_UP",
            Self::ThumbsDown => "THUMBS_DOWN",
            Self::Laugh => "LAUGH",
            Self::Hooray => "HOORAY",
            Self::Confused => "CONFUSED",
            Self::Heart => "HEART",
            Self::Rocket => "ROCKET",
            Self::Eyes => "EYES",
        }
    }

    /// GitLab award emoji name.
    pub fn gitlab(self) -> &'static str {
        match self {
            Self::ThumbsUp => "thumbsup",
            Self::ThumbsDown => "thumbsdown",
            Self::Laugh => "smile",
            Self::Hooray => "tada",
            Self::Confused => "confused",
            Self::Heart => "heart",
            Self::Rocket => "rocket",
            Self::Eyes => "eyes",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            Self::ThumbsUp => "\u{1F44D}",
            Self::ThumbsDown => "\u{1F44E}",
            Self::Laugh => "\u{1F604}",
            Self::Hooray => "\u{1F389}",
            Self::Confused => "\u{1F615}",
            Self::Heart => "\u{2764}\u{FE0F}",
            Self::Rocket => "\u{1F680}",
            Self::Eyes => "\u{1F440}",
        }
    }

    /// Parse any of the provider spellings above.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|k| k.github_rest() == s || k.github_graphql() == s || k.gitlab() == s)
    }
}

/// One reaction with its count, whether I gave it and (some of) who did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionGroup {
    pub kind: ReactionKind,
    pub count: u32,
    pub me: bool,
    #[serde(default)]
    pub users: Vec<String>,
}

/// Group a flat list of (kind, username) award emoji, keeping `ALL` order and dropping unknown names.
pub fn group_reactions<'a>(
    items: impl IntoIterator<Item = (&'a str, &'a str)>,
    me: &str,
) -> Vec<ReactionGroup> {
    let mut out: Vec<ReactionGroup> = vec![];
    for (name, user) in items {
        let Some(kind) = ReactionKind::parse(name) else {
            continue;
        };
        match out.iter_mut().find(|g| g.kind == kind) {
            Some(g) => {
                g.count += 1;
                g.me |= user == me;
                g.users.push(user.to_string());
            }
            None => out.push(ReactionGroup {
                kind,
                count: 1,
                me: user == me,
                users: vec![user.to_string()],
            }),
        }
    }
    out.sort_by_key(|g| ReactionKind::ALL.iter().position(|k| *k == g.kind));
    out
}

/// Add or remove my reaction in place (optimistic update).
pub fn toggle_reaction(groups: &mut Vec<ReactionGroup>, kind: ReactionKind, me: &str) -> bool {
    match groups.iter().position(|g| g.kind == kind) {
        Some(i) if groups[i].me => {
            let g = &mut groups[i];
            g.me = false;
            g.count = g.count.saturating_sub(1);
            g.users.retain(|u| u != me);
            if g.count == 0 {
                groups.remove(i);
            }
            false
        }
        Some(i) => {
            let g = &mut groups[i];
            g.me = true;
            g.count += 1;
            g.users.push(me.to_string());
            true
        }
        None => {
            groups.push(ReactionGroup {
                kind,
                count: 1,
                me: true,
                users: vec![me.to_string()],
            });
            groups.sort_by_key(|g| ReactionKind::ALL.iter().position(|k| *k == g.kind));
            true
        }
    }
}

/// One commit of a PR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub at: Option<DateTime<Utc>>,
    pub url: Option<String>,
}

impl CommitInfo {
    pub fn short(&self) -> &str {
        &self.sha[..self.sha.len().min(7)]
    }
}

/// Commits after my last reviewed commit, or None when I never reviewed (or it is unknown).
pub fn commits_since(commits: &[CommitInfo], reviewed_sha: Option<&str>) -> Option<usize> {
    let sha = reviewed_sha?;
    let i = commits.iter().position(|c| c.sha == sha)?;
    Some(commits.len() - 1 - i)
}

/// Fingerprint of a file's diff, so a locally stored "viewed" mark resets when the file changes.
pub fn file_fingerprint(f: &crate::ChangedFile) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    f.new_path.hash(&mut h);
    for hunk in &f.hunks {
        hunk.header.hash(&mut h);
        for l in &hunk.lines {
            l.text.hash(&mut h);
        }
    }
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_provider_names() {
        for k in ReactionKind::ALL {
            assert_eq!(ReactionKind::parse(k.github_rest()), Some(k));
            assert_eq!(ReactionKind::parse(k.github_graphql()), Some(k));
            assert_eq!(ReactionKind::parse(k.gitlab()), Some(k));
        }
        assert_eq!(ReactionKind::parse("+1"), Some(ReactionKind::ThumbsUp));
        assert_eq!(ReactionKind::parse("tada"), Some(ReactionKind::Hooray));
        assert_eq!(ReactionKind::parse("smile"), Some(ReactionKind::Laugh));
        assert_eq!(ReactionKind::parse("100"), None);
    }

    #[test]
    fn groups_and_toggles() {
        let mut g = group_reactions(
            [
                ("heart", "bo"),
                ("thumbsup", "me"),
                ("thumbsup", "ada"),
                ("fire", "x"),
            ],
            "me",
        );
        assert_eq!(g.len(), 2);
        assert_eq!(
            (g[0].kind, g[0].count, g[0].me),
            (ReactionKind::ThumbsUp, 2, true)
        );
        assert!(!toggle_reaction(&mut g, ReactionKind::ThumbsUp, "me"));
        assert_eq!((g[0].count, g[0].me), (1, false));
        assert!(toggle_reaction(&mut g, ReactionKind::Eyes, "me"));
        assert!(toggle_reaction(&mut g, ReactionKind::Heart, "me"));
        assert_eq!(
            g.iter()
                .find(|x| x.kind == ReactionKind::Heart)
                .map(|x| x.count),
            Some(2)
        );
        let mut solo = vec![];
        toggle_reaction(&mut solo, ReactionKind::Rocket, "me");
        toggle_reaction(&mut solo, ReactionKind::Rocket, "me");
        assert!(solo.is_empty());
    }

    #[test]
    fn counts_commits_since_review() {
        let c = |s: &str| CommitInfo {
            sha: s.into(),
            message: String::new(),
            author: String::new(),
            at: None,
            url: None,
        };
        let cs = vec![c("a"), c("b"), c("c")];
        assert_eq!(commits_since(&cs, Some("a")), Some(2));
        assert_eq!(commits_since(&cs, Some("c")), Some(0));
        assert_eq!(commits_since(&cs, Some("z")), None);
        assert_eq!(commits_since(&cs, None), None);
    }
}
