//! The demo pull and merge requests (summaries).

use crate::people::{user, ME};
use chrono::{Duration, Utc};
use gv_core::*;

pub const GITHUB: &str = "github.com";
pub const GITLAB: &str = "gitlab.com";

/// Repos per host: (kind, host, path).
pub const REPOS: &[(ProviderKind, &str, &str)] = &[
    (ProviderKind::GitHub, GITHUB, "acme/web"),
    (ProviderKind::GitHub, GITHUB, "acme/api"),
    (ProviderKind::GitHub, GITHUB, "acme/design-tokens"),
    (ProviderKind::GitLab, GITLAB, "orbit/cli"),
    (ProviderKind::GitLab, GITLAB, "orbit/platform/ingest"),
];

use ChangeRequestState::{Draft, Merged, Open};
use ReviewState::{Approved as A, ChangesRequested as X, Commented as C, Pending as P};

/// (repo index, number, title, author, state, branch, reviewers, checks passed/failed/running,
/// labels, comments, +, -, files, minutes ago)
type Spec = (usize, u64, &'static str, &'static str, ChangeRequestState, &'static str, &'static [(&'static str, ReviewState)], (u32, u32, u32), &'static [&'static str], u32, u32, u32, u32, i64);

const SPECS: &[Spec] = &[
    (1, 312, "Retry webhook deliveries with exponential backoff", "linus-k", Open, "linus/webhook-retries", &[(ME, P), ("ada", A), ("grace.h", C)], (6, 0, 1), &["backend", "reliability"], 7, 214, 38, 4, 12),
    (0, 1287, "Virtualize the activity feed for long histories", "ada", Open, "ada/virtual-feed", &[(ME, P), ("tomas", A)], (9, 0, 0), &["frontend", "performance"], 4, 168, 92, 3, 34),
    (3, 58, "Add `orbit deploy --dry-run` with a plan preview", "grace.h", Open, "grace/dry-run", &[(ME, P), ("priya.r", P)], (4, 1, 0), &["cli"], 3, 241, 17, 5, 55),
    (0, 1291, "Fix focus ring clipping in the settings sidebar", ME, Open, "mira/focus-ring", &[("ada", X)], (9, 0, 0), &["a11y"], 5, 12, 9, 2, 80),
    (1, 318, "Stream search results over server-sent events", ME, Open, "mira/sse-search", &[("linus-k", A), ("noor", A)], (5, 2, 0), &["backend"], 2, 187, 44, 6, 140),
    (4, 211, "Batch Kafka offsets commits per partition", "priya.r", Open, "priya/batch-offsets", &[(ME, C), ("linus-k", P)], (3, 0, 2), &["ingest"], 6, 96, 31, 3, 190),
    (2, 77, "Introduce elevation tokens for dark surfaces", "tomas", Open, "tomas/elevation", &[(ME, A)], (4, 0, 0), &["design"], 1, 58, 12, 4, 260),
    (0, 1294, "Prefetch PR details on hover", ME, Draft, "mira/prefetch", &[], (0, 0, 3), &["frontend"], 0, 73, 8, 2, 300),
    (3, 61, "Shell completions for zsh and fish", "noor", Open, "noor/completions", &[(ME, P)], (5, 0, 0), &["cli", "dx"], 2, 132, 0, 3, 410),
    (1, 309, "Rate limit API tokens per organization", "grace.h", Open, "grace/org-limits", &[("linus-k", X), (ME, A)], (6, 0, 0), &["backend", "security"], 9, 305, 120, 8, 600),
    (0, 1280, "chore(deps): update dependency vite to v6.3.1", "renovate[bot]", Open, "renovate/vite-6.x", &[], (9, 0, 0), &["dependencies"], 0, 14, 14, 2, 720),
    (1, 315, "chore(deps): update rust crate tokio to 1.44", "renovate[bot]", Open, "renovate/tokio-1.x", &[], (6, 0, 0), &["dependencies"], 0, 6, 6, 2, 760),
    (4, 205, "Drop events older than the retention window", "linus-k", Open, "linus/retention", &[(ME, P), ("priya.r", A)], (4, 0, 0), &["ingest"], 3, 64, 22, 2, 1500),
    (3, 55, "Respect NO_COLOR in all output paths", ME, Merged, "mira/no-color", &[("grace.h", A)], (5, 0, 0), &["cli"], 2, 41, 19, 4, 2900),
    (2, 74, "Rename `space.*` tokens to the 4pt scale", "ada", Open, "ada/space-scale", &[("tomas", P)], (4, 0, 0), &["design"], 1, 88, 88, 6, 3100),
    (0, 1270, "Command palette: fuzzy match on PR numbers", "noor", Open, "noor/palette-numbers", &[(ME, A), ("ada", A)], (9, 0, 0), &["frontend"], 4, 77, 15, 3, 4300),
];

fn web_url(repo: &RepoRef, n: u64) -> String {
    match repo.kind {
        ProviderKind::GitLab => format!("https://{}/{}/-/merge_requests/{n}", repo.host, repo.path),
        _ => format!("https://{}/{}/pull/{n}", repo.host, repo.path),
    }
}

fn build(s: &Spec) -> ChangeRequest {
    let (ri, number, title, author, state, branch, reviewers, (p, f, r), labels, comments, add, del, files, mins) = *s;
    let (kind, host, path) = REPOS[ri];
    let repo = RepoRef::new(kind, host, path);
    let updated = Utc::now() - Duration::minutes(mins);
    let mut cr = ChangeRequest {
        web_url: web_url(&repo, number),
        id: ChangeRequestId { repo, number },
        title: title.into(),
        author: user(author),
        state,
        source_branch: branch.into(),
        target_branch: "main".into(),
        reviewers: reviewers.iter().map(|(u, st)| Review { user: user(u), state: *st }).collect(),
        checks: CheckSummary { total: p + f + r, passed: p, failed: f, pending: r },
        labels: labels.iter().map(|l| l.to_string()).collect(),
        comments_count: comments,
        additions: Some(add),
        deletions: Some(del),
        changed_files: Some(files),
        updated_at: updated,
        created_at: updated - Duration::hours(26 + mins / 60),
        needs_you: false,
    };
    cr.needs_you = compute_needs_you(&cr, ME);
    cr
}

/// Every demo PR/MR, newest first.
pub fn all() -> Vec<ChangeRequest> {
    SPECS.iter().map(build).collect()
}

/// The PRs/MRs on one host.
pub fn on_host(kind: ProviderKind, host: &str) -> Vec<ChangeRequest> {
    all().into_iter().filter(|c| c.id.repo.kind == kind && c.id.repo.host == host).collect()
}

/// Repos under `owner` on a host.
pub fn repos(kind: ProviderKind, host: &str, owner: &str) -> Vec<RepoRef> {
    let prefix = format!("{owner}/");
    REPOS.iter().filter(|(k, h, p)| *k == kind && *h == host && p.starts_with(&prefix)).map(|(k, h, p)| RepoRef::new(*k, *h, *p)).collect()
}

pub fn find(id: &ChangeRequestId) -> Option<ChangeRequest> {
    all().into_iter().find(|c| &c.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixes_states_and_providers() {
        let all = all();
        assert!(all.len() >= 15);
        assert!(all.iter().any(|c| c.id.repo.kind == ProviderKind::GitLab));
        assert!(all.iter().any(|c| c.needs_you));
        assert!(all.iter().any(|c| c.author.bot));
        assert_eq!(repos(ProviderKind::GitLab, GITLAB, "orbit").len(), 2);
        let first = &all[0];
        assert_eq!(find(&first.id).unwrap().title, first.title);
    }
}
