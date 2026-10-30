//! Full detail of a demo PR: description, threads, timeline, checks and commits.

use crate::people::{user, ME};
use crate::threads;
use chrono::{Duration, Utc};
use gv_core::*;

const DESCRIPTION: &str = include_str!("fixtures/description.md");

/// The PR with the richest fixture (threads, table, code block).
pub fn is_showcase(id: &ChangeRequestId) -> bool {
    id.repo.path == "acme/api" && id.number == 312
}

fn checks(cr: &ChangeRequest) -> ChecksInfo {
    let names = ["build", "test (stable)", "test (beta)", "clippy", "fmt", "audit", "e2e", "docs", "coverage", "deploy-preview"];
    let s = cr.checks;
    let started = Utc::now() - Duration::minutes(18);
    let runs = names
        .iter()
        .take(s.total as usize)
        .enumerate()
        .map(|(i, n)| {
            let i = i as u32;
            let status = if i < s.failed {
                CheckStatus::Failed
            } else if i < s.failed + s.pending {
                CheckStatus::Running
            } else {
                CheckStatus::Passed
            };
            let done = status != CheckStatus::Running;
            CheckRun {
                id: format!("{}", 9000 + i),
                name: n.to_string(),
                stage: Some(if cr.id.repo.kind == ProviderKind::GitLab { "test".into() } else { "CI".into() }),
                status,
                started_at: Some(started),
                finished_at: done.then(|| started + Duration::seconds(40 + 37 * i as i64)),
                url: None,
                has_log: status == CheckStatus::Failed,
            }
        })
        .collect();
    ChecksInfo { runs, rerun_target: (s.failed > 0).then(|| "1".into()), can_rerun: s.failed > 0 }
}

fn commits(cr: &ChangeRequest) -> Vec<CommitInfo> {
    let msgs = ["Add retry policy type", "Retry transient delivery failures", "Cover delays with tests"];
    msgs.iter()
        .enumerate()
        .map(|(i, m)| CommitInfo { sha: format!("{:x}{:032x}", 0xa1b2c3d4u32 + i as u32, i), message: m.to_string(), author: cr.author.username.clone(), at: Some(cr.created_at + Duration::hours(i as i64)), url: None })
        .collect()
}

fn events(cr: &ChangeRequest) -> Vec<TimelineEvent> {
    let mut v = vec![TimelineEvent { kind: TimelineKind::Opened, actor: cr.author.clone(), at: cr.created_at }];
    for (i, r) in cr.reviewers.iter().filter(|r| r.state != ReviewState::Pending).enumerate() {
        v.push(TimelineEvent { kind: TimelineKind::Reviewed(r.state), actor: r.user.clone(), at: cr.updated_at - Duration::minutes(30 * (i as i64 + 1)) });
    }
    if cr.state == ChangeRequestState::Merged {
        v.push(TimelineEvent { kind: TimelineKind::Merged, actor: user("grace.h"), at: cr.updated_at });
    }
    v
}

fn generic_description(cr: &ChangeRequest) -> String {
    format!("{}.\n\n- Keeps the public API unchanged\n- Adds tests for the new paths\n\nCloses #{}.", cr.title.trim_end_matches('.'), cr.id.number - 7)
}

/// Build the detail of `cr`.
pub fn build(cr: ChangeRequest) -> ChangeRequestDetail {
    let showcase = is_showcase(&cr.id);
    let threads = if showcase {
        threads::showcase()
    } else {
        threads::generic(&cr.author.username, cr.reviewers.first().map(|r| r.user.username.as_str()))
    };
    let caps = match cr.id.repo.kind {
        ProviderKind::GitLab => gitlab_capabilities(&cr, ME, false, true),
        _ => github_capabilities(&cr, ME),
    };
    let mut edit = EditInfo { caps: EditCapabilities::editor(cr.state.is_active()), ..Default::default() };
    edit.caps.merge = cr.state == ChangeRequestState::Open;
    edit.caps.merge_methods = vec![MergeMethod::Squash, MergeMethod::Merge];
    ChangeRequestDetail {
        description: if showcase { DESCRIPTION.into() } else { generic_description(&cr) },
        threads,
        head_sha: Some(format!("{:040x}", cr.id.number * 7919)),
        review: ReviewInfo { caps, my_state: my_review_state(&cr, ME), approvals: None },
        events: events(&cr),
        edit,
        checks: checks(&cr),
        reactions: if showcase { vec![ReactionGroup { kind: ReactionKind::Hooray, count: 2, me: false, users: vec!["ada".into(), "noor".into()] }] } else { vec![] },
        commits: commits(&cr),
        my_review_sha: None,
        viewed: None,
        node_id: None,
        summary: cr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showcase_has_every_thread_kind() {
        let cr = crate::catalog::all().into_iter().find(|c| is_showcase(&c.id)).unwrap();
        let d = build(cr);
        let att: Vec<Attention> = d.threads.iter().map(|t| thread_attention(t, Some(ME), true)).collect();
        assert!(att.contains(&Attention::NeedsMe));
        assert!(att.contains(&Attention::Open));
        assert!(att.contains(&Attention::Resolved));
        assert!(d.description.contains("```rust"));
        assert_eq!(d.checks.count(CheckStatus::Running), 1);
    }
}
