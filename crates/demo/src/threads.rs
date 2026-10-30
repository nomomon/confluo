//! Conversation and inline review threads of the demo PRs.

use crate::diffs::{DELIVER_PATH, RETRY_PATH};
use crate::people::user;
use chrono::{Duration, Utc};
use gv_core::*;

fn c(
    id: &str,
    who: &str,
    mins: i64,
    body: &str,
    pos: Option<(&str, u32)>,
    reactions: Vec<ReactionGroup>,
) -> Comment {
    Comment {
        id: id.into(),
        author: user(who),
        body: body.into(),
        created_at: Utc::now() - Duration::minutes(mins),
        position: pos.map(|(p, l)| InlinePosition {
            path: p.into(),
            line: Some(l),
        }),
        reactions,
        suggestions: vec![],
    }
}

fn r(kind: ReactionKind, users: &[&str]) -> ReactionGroup {
    ReactionGroup {
        kind,
        count: users.len() as u32,
        me: users.contains(&crate::people::ME),
        users: users.iter().map(|u| u.to_string()).collect(),
    }
}

fn inline(id: &str, path: &str, line: u32, resolved: bool, comments: Vec<Comment>) -> Thread {
    Thread {
        id: id.into(),
        resolved: Some(resolved),
        comments,
        position: Some(DiffPosition::new(path, DiffSide::New, line)),
        outdated: false,
        resolved_by: resolved.then(|| user("ada")),
        resolved_at: resolved.then(|| Utc::now() - Duration::minutes(250)),
    }
}

/// A suggested change (GitHub/GitLab fence) with a provider handle so "Apply suggestion" shows.
fn suggested(mut cm: Comment) -> Comment {
    cm.suggestions = vec![SuggestionRef {
        id: "901".into(),
        appliable: true,
        applied: false,
    }];
    cm
}

/// Threads of the showcase PR (webhook retries).
pub fn showcase() -> Vec<Thread> {
    vec![
        Thread::conversation("c1", vec![c("c1", "ada", 300, "Replayed yesterday's staging traffic against a receiver that fails 30% of the time: **0 dropped events**, p99 delivery latency went from 0.4s to 2.1s, which is fine for webhooks.", None, vec![r(ReactionKind::ThumbsUp, &["linus-k", "grace.h", "mira"]), r(ReactionKind::Rocket, &["linus-k"])])]),
        inline("t1", RETRY_PATH, 28, false, vec![
            c("t1a", "grace.h", 240, "Full jitter can produce a zero delay. Do we want a floor of `base / 2` so a retry storm cannot hit the receiver immediately?", Some((RETRY_PATH, 28)), vec![]),
            c("t1b", "linus-k", 200, "@mira you wrote the original delivery loop, what do you think? I lean towards keeping full jitter, it spreads load better.", Some((RETRY_PATH, 28)), vec![r(ReactionKind::Eyes, &["grace.h"])]),
            c("t1c", "ada", 180, "Full jitter matches what the queue workers do, I'd keep it consistent.", Some((RETRY_PATH, 28)), vec![]),
        ]),
        inline("t2", DELIVER_PATH, 16, false, vec![
            c("t2a", "mira", 150, "`Retry-After` on a 429 should win over our own delay, otherwise we retry too early.", Some((DELIVER_PATH, 16)), vec![]),
            c("t2b", "linus-k", 120, "Good call, adding it in a follow-up commit.", Some((DELIVER_PATH, 16)), vec![r(ReactionKind::ThumbsUp, &["mira"])]),
            c("t2c", "linus-k", 90, "Pushed: `deliver` now reads `Retry-After` (seconds or HTTP date) and caps it at `policy.cap`.", Some((DELIVER_PATH, 16)), vec![]),
            c("t2d", "mira", 70, "Sounds good, I'll re-review once CI is green.", Some((DELIVER_PATH, 16)), vec![]),
        ]),
        inline("t4", DELIVER_PATH, 7, false, vec![
            suggested(c("t4a", "lintbot", 230, "**Clarity:** `RetryPolicy::delay` takes a `u32`. Spell out the counter's type so it never gets inferred wider.\n\n```suggestion\n    let mut attempt: u32 = 0;\n```", Some((DELIVER_PATH, 7)), vec![])),
        ]),
        inline("t3", RETRY_PATH, 16, true, vec![
            c("t3a", "ada", 280, "nit: 8 attempts with a 5 minute cap is about 20 minutes total. Worth a comment?", Some((RETRY_PATH, 16)), vec![]),
            c("t3b", "linus-k", 260, "Added it to the PR description table.", Some((RETRY_PATH, 16)), vec![r(ReactionKind::Heart, &["ada"])]),
        ]),
        Thread::conversation("c2", vec![c("c2", "grace.h", 60, "Looks close. One open question on the jitter floor, otherwise good from my side.", None, vec![])]),
    ]
}

/// Threads of any other PR: one friendly comment.
pub fn generic(author: &str, reviewer: Option<&str>) -> Vec<Thread> {
    let who = reviewer.unwrap_or(author);
    vec![Thread::conversation(
        "g1",
        vec![c(
            "g1",
            who,
            45,
            "Thanks, this reads well. Left one question inline in the next round.",
            None,
            vec![r(ReactionKind::ThumbsUp, &[author])],
        )],
    )]
}
