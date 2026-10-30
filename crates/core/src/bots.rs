//! Which change requests come from bots (Renovate, Dependabot, project bots) and are hidden.

use crate::ChangeRequest;
use serde::{Deserialize, Serialize};

/// Author patterns hidden by default. `*` matches any run of characters; case-insensitive.
pub const DEFAULT_BOT_AUTHORS: &[&str] = &[
    "renovate",
    "renovate-bot",
    "dependabot",
    "*[bot]",
    "*-bot",
    "project_*_bot*",
    "group_*_bot*",
];
/// Source branch prefixes hidden by default.
pub const DEFAULT_BOT_BRANCHES: &[&str] = &["renovate/", "dependabot/"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BotFilter {
    /// Hide bot PRs at all.
    pub enabled: bool,
    /// Author username patterns.
    pub authors: Vec<String>,
    /// Source branch prefixes.
    pub branch_prefixes: Vec<String>,
}

impl Default for BotFilter {
    fn default() -> Self {
        Self {
            enabled: true,
            authors: DEFAULT_BOT_AUTHORS.iter().map(|s| s.to_string()).collect(),
            branch_prefixes: DEFAULT_BOT_BRANCHES.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl BotFilter {
    /// A filter that hides nothing.
    pub fn off() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Whether `cr` looks like a bot PR, regardless of `enabled`.
    pub fn is_bot(&self, cr: &ChangeRequest) -> bool {
        let a = &cr.author;
        a.bot
            || a.username.to_ascii_lowercase().ends_with("[bot]")
            || self.authors.iter().any(|p| glob(p.trim(), &a.username))
            || self.branch_prefixes.iter().map(|p| p.trim()).any(|p| {
                !p.is_empty()
                    && cr
                        .source_branch
                        .to_ascii_lowercase()
                        .starts_with(&p.to_ascii_lowercase())
            })
    }

    /// Whether `cr` is hidden by this filter.
    pub fn hides(&self, cr: &ChangeRequest) -> bool {
        self.enabled && self.is_bot(cr)
    }
}

/// Case-insensitive match where `*` matches any run of characters. An empty pattern matches nothing.
pub fn glob(pattern: &str, s: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = s.to_lowercase().chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while ti < t.len() {
        if pi < p.len() && p[pi] != '*' && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(sp) = star {
            pi = sp + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn cr(author: &str, branch: &str, bot: bool) -> ChangeRequest {
        ChangeRequest {
            id: ChangeRequestId {
                repo: RepoRef::new(ProviderKind::GitLab, "gitlab.com", "g/p"),
                number: 1,
            },
            title: "t".into(),
            author: User {
                bot,
                ..User::named(author)
            },
            state: ChangeRequestState::Open,
            source_branch: branch.into(),
            target_branch: "main".into(),
            reviewers: vec![],
            checks: CheckSummary::default(),
            labels: vec![],
            comments_count: 0,
            additions: None,
            deletions: None,
            changed_files: None,
            updated_at: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
            web_url: String::new(),
            needs_you: false,
            signals: Default::default(),
        }
    }

    #[test]
    fn globs() {
        assert!(glob("*-bot", "acme-bot"));
        assert!(glob("project_*_bot*", "project_123_bot_4f2a"));
        assert!(glob("Renovate", "renovate"));
        assert!(!glob("renovate", "renovate-bot"));
        assert!(!glob("*-bot", "abbot"));
        assert!(!glob("", "x"));
        assert!(glob("*", "x"));
    }

    #[test]
    fn matches_bots() {
        let f = BotFilter::default();
        assert!(f.hides(&cr("acme-bot", "renovate/react", false)));
        assert!(f.hides(&cr("dependabot[bot]", "x", false)));
        assert!(f.hides(&cr("copilot", "x", true)));
        assert!(f.hides(&cr("project_42_bot_abc", "x", false)));
        assert!(f.hides(&cr("alice", "renovate/eslint", false)));
        assert!(f.hides(&cr("alice", "Dependabot/npm/x", false)));
        assert!(!f.hides(&cr("alice", "feat/renovate-config", false)));
        assert!(!f.hides(&cr("robot", "main", false)));
        assert!(!BotFilter::off().hides(&cr("renovate", "renovate/x", false)));
        assert!(BotFilter::off().is_bot(&cr("renovate", "x", false)));
    }

    #[test]
    fn custom_lists() {
        let f = BotFilter {
            enabled: true,
            authors: vec!["ci-*".into()],
            branch_prefixes: vec![],
        };
        assert!(f.hides(&cr("ci-runner", "x", false)));
        assert!(!f.hides(&cr("acme-bot", "renovate/x", false)));
    }
}
