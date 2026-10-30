//! Editing a PR/MR: title, body, people, draft, state and merge, and which of those I may do.

use crate::User;
use serde::{Deserialize, Serialize};

/// Who a people list is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PeopleRole {
    Reviewers,
    Assignees,
}

impl PeopleRole {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Reviewers => "Reviewers",
            Self::Assignees => "Assignees",
        }
    }
}

/// A merge strategy the repo allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
    /// Azure DevOps "semi-linear merge": rebase, then a merge commit.
    RebaseMerge,
}

impl MergeMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Merge => "Create a merge commit",
            Self::Squash => "Squash and merge",
            Self::Rebase => "Rebase and merge",
            Self::RebaseMerge => "Rebase with merge commit",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
            Self::RebaseMerge => "rebase-merge",
        }
    }
}

/// What the merge dialog submits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeOptions {
    pub method: MergeMethod,
    /// GitLab: squash commits (with `method == Merge`).
    pub squash: bool,
    pub delete_branch: bool,
    /// GitLab: merge once the pipeline succeeds.
    pub when_pipeline_succeeds: bool,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self { method: MergeMethod::Merge, squash: false, delete_branch: false, when_pipeline_succeeds: false }
    }
}

/// Edit actions the provider allows on one PR for the signed-in user.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditCapabilities {
    pub title: bool,
    pub body: bool,
    pub reviewers: bool,
    pub assignees: bool,
    pub draft: bool,
    pub close: bool,
    pub reopen: bool,
    pub merge: bool,
    /// Allowed merge methods, in display order; empty when merging is not possible.
    pub merge_methods: Vec<MergeMethod>,
    /// GitLab offers a separate squash checkbox.
    pub squash_option: bool,
    /// Squash is preselected (GitLab project or MR setting).
    pub squash_default: bool,
    pub delete_branch_option: bool,
    pub delete_branch_default: bool,
    /// The pipeline is still running, so "merge when it succeeds" is possible.
    pub auto_merge: bool,
    /// Why merging is not possible right now (conflicts, draft, checks).
    pub merge_blocked: Option<String>,
}

impl EditCapabilities {
    /// Every edit allowed (author or maintainer on an open PR), without merge.
    pub fn editor(open: bool) -> Self {
        Self { title: true, body: true, reviewers: open, assignees: true, draft: open, close: open, reopen: !open, ..Default::default() }
    }
}

/// Edit side of a PR detail.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditInfo {
    pub caps: EditCapabilities,
    pub assignees: Vec<User>,
    /// Provider node id (GitHub GraphQL mutations need it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
}

/// `Draft: ` prefix handling for GitLab, which marks drafts by title.
pub fn draft_title(title: &str, draft: bool) -> String {
    let mut bare = title.trim_start();
    loop {
        let lower = bare.to_ascii_lowercase();
        let cut = ["draft:", "[draft]", "(draft)", "wip:", "[wip]"].iter().find(|p| lower.starts_with(*p)).map(|p| p.len());
        match cut {
            Some(n) => bare = bare[n..].trim_start(),
            None => break,
        }
    }
    if draft {
        format!("Draft: {bare}")
    } else {
        bare.to_string()
    }
}

/// Usernames to add and remove to go from `current` to `desired` (case-insensitive).
pub fn people_diff(current: &[String], desired: &[String]) -> (Vec<String>, Vec<String>) {
    let has = |list: &[String], u: &str| list.iter().any(|x| x.eq_ignore_ascii_case(u));
    let add = desired.iter().filter(|u| !has(current, u)).cloned().collect();
    let remove = current.iter().filter(|u| !has(desired, u)).cloned().collect();
    (add, remove)
}

/// Apply `+user -user user` CLI arguments to a list of usernames.
pub fn apply_people_args(current: &[String], args: &[String]) -> Vec<String> {
    let mut out: Vec<String> = current.to_vec();
    for a in args {
        if let Some(u) = a.strip_prefix('-') {
            out.retain(|x| !x.eq_ignore_ascii_case(u));
        } else {
            let u = a.strip_prefix('+').unwrap_or(a).trim_start_matches('@');
            if !u.is_empty() && !out.iter().any(|x| x.eq_ignore_ascii_case(u)) {
                out.push(u.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_prefix() {
        assert_eq!(draft_title("Fix it", true), "Draft: Fix it");
        assert_eq!(draft_title("Draft: Fix it", true), "Draft: Fix it");
        assert_eq!(draft_title("draft: [WIP] Fix it", false), "Fix it");
        assert_eq!(draft_title("Drafting docs", false), "Drafting docs");
    }

    #[test]
    fn people() {
        let cur = vec!["ada".to_string(), "bob".to_string()];
        let want = apply_people_args(&cur, &["+Carol".into(), "-bob".into(), "@ada".into()]);
        assert_eq!(want, vec!["ada", "Carol"]);
        assert_eq!(people_diff(&cur, &want), (vec!["Carol".to_string()], vec!["bob".to_string()]));
    }
}
