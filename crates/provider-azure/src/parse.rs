//! Mapping Azure DevOps JSON (pull requests, identities, threads, commits) to core types.

use chrono::{DateTime, Utc};
use gv_core::*;
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use serde_json::Value;

/// Characters escaped in one URL path segment.
const SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'?')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'{')
    .add(b'}');
/// Characters escaped in a query value.
const QUERY: &AsciiSet = &SEGMENT.add(b'&').add(b'=').add(b'+').add(b':').add(b'\'');

/// One path segment (names may contain spaces; remotes may already be %-encoded).
pub(crate) fn seg(s: &str) -> String {
    let decoded = percent_decode_str(s).decode_utf8_lossy();
    utf8_percent_encode(&decoded, SEGMENT).to_string()
}

pub(crate) fn query(s: &str) -> String {
    utf8_percent_encode(s, QUERY).to_string()
}

/// `org/project/repo` -> its three parts.
pub(crate) fn split_repo(path: &str) -> Option<(&str, &str, &str)> {
    let mut it = path.splitn(3, '/');
    let (o, p, r) = (it.next()?, it.next()?, it.next()?);
    (!o.is_empty() && !p.is_empty() && !r.is_empty() && !r.contains('/')).then_some((o, p, r))
}

pub(crate) fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

pub(crate) fn time(v: &Value) -> DateTime<Utc> {
    v.as_str()
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_default()
}

pub(crate) fn opt_time(v: &Value) -> Option<DateTime<Utc>> {
    v.as_str()
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&Utc))
        .filter(|t| t.timestamp() > 0)
}

/// A group reviewer: `isContainer`, or a `[Project]\Team` name.
fn is_group(v: &Value, display: &str) -> bool {
    v["isContainer"] == true || (display.starts_with('[') && display.contains("]\\"))
}

/// An `IdentityRef`. Username is the unique name (usually the email); groups are `team:[Project]\Team`
/// named "Team". The avatar is the identity image (fetched with the token).
pub(crate) fn user(v: &Value) -> User {
    let display = clean_display(v["displayName"].as_str().unwrap_or("unknown"));
    let unique = v["uniqueName"]
        .as_str()
        .filter(|u| !u.is_empty() && !u.starts_with("vstfs:"));
    let group = is_group(v, display);
    let (username, name) = if group {
        let short = display.rsplit('\\').next().unwrap_or(display);
        (format!("team:{display}"), short)
    } else {
        (unique.unwrap_or(display).to_string(), display)
    };
    User {
        username,
        name: Some(name.to_string()),
        avatar_url: v["imageUrl"]
            .as_str()
            .or(v["_links"]["avatar"]["href"].as_str())
            .filter(|u| !group && u.starts_with("https://"))
            .map(str::to_string),
        bot: unique.is_some_and(|u| u.starts_with("Build\\"))
            || display.ends_with("Build Service")
            || display.contains("Build Service ("),
        id: v["id"].as_str().map(str::to_string),
    }
}

pub(crate) fn same(u: &User, me: &User) -> bool {
    match (&u.id, &me.id) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => u.username.eq_ignore_ascii_case(&me.username),
    }
}

/// The user from `GET {org}/_apis/connectionData`.
pub(crate) fn connection_user(v: &Value) -> Option<User> {
    let a = &v["authenticatedUser"];
    let id = a["id"].as_str()?;
    let display = a["providerDisplayName"]
        .as_str()
        .or(a["customDisplayName"].as_str())
        .unwrap_or("me");
    let mail = a["properties"]["Account"]["$value"]
        .as_str()
        .filter(|m| m.contains('@'));
    Some(User {
        username: mail.unwrap_or(display).to_string(),
        name: Some(display.to_string()),
        avatar_url: None,
        bot: false,
        id: Some(id.to_string()),
    })
}

/// The user from `GET app.vssps.visualstudio.com/_apis/profile/profiles/me`.
pub(crate) fn profile_user(v: &Value) -> Option<User> {
    let id = v["id"].as_str()?;
    let display = v["displayName"].as_str().unwrap_or("me");
    let mail = v["emailAddress"].as_str().filter(|m| !m.is_empty());
    Some(User {
        username: mail.unwrap_or(display).to_string(),
        name: Some(display.to_string()),
        avatar_url: None,
        bot: false,
        id: Some(id.to_string()),
    })
}

/// An identity from `vssps.dev.azure.com/{org}/_apis/identities` (reviewer search).
pub(crate) fn identity_user(v: &Value) -> Option<User> {
    if v["isActive"] == false || v["isContainer"] == true {
        return None;
    }
    let id = v["id"].as_str()?;
    let display = v["providerDisplayName"]
        .as_str()
        .or(v["customDisplayName"].as_str())
        .unwrap_or("unknown");
    let mail = v["properties"]["Mail"]["$value"]
        .as_str()
        .or(v["properties"]["Account"]["$value"].as_str())
        .filter(|m| !m.is_empty());
    Some(User {
        username: mail.unwrap_or(display).to_string(),
        name: Some(display.to_string()),
        avatar_url: None,
        bot: false,
        id: Some(id.to_string()),
    })
}

/// Reviewer vote -> review state: 10 approved, 5 approved with suggestions, -5 waiting for the author,
/// -10 rejected, 0 no vote.
pub fn vote_state(vote: i64) -> ReviewState {
    match vote {
        v if v >= 5 => ReviewState::Approved,
        v if v <= -5 => ReviewState::ChangesRequested,
        _ => ReviewState::Pending,
    }
}

/// The vote a review action casts; None for actions Azure DevOps has no vote for.
pub fn vote_action(action: ReviewAction) -> Option<i64> {
    match action {
        ReviewAction::Approve => Some(10),
        ReviewAction::ApproveWithSuggestions => Some(5),
        ReviewAction::ResetVote | ReviewAction::Unapprove => Some(0),
        ReviewAction::WaitForAuthor => Some(-5),
        ReviewAction::RequestChanges => Some(-10),
        ReviewAction::Comment => None,
    }
}

/// My vote on a PR payload (0 when I am not a reviewer).
pub(crate) fn my_vote(pr: &Value, me: &User) -> i64 {
    pr["reviewers"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|r| same(&user(r), me))
        .and_then(|r| r["vote"].as_i64())
        .unwrap_or(0)
}

fn branch(v: &Value) -> String {
    let r = v.as_str().unwrap_or_default();
    r.strip_prefix("refs/heads/").unwrap_or(r).to_string()
}

/// A `GitPullRequest` (list or single) in organization `org`.
pub(crate) fn parse_pr(host: &str, org: &str, v: &Value, me: &User) -> ChangeRequest {
    let project = v["repository"]["project"]["name"]
        .as_str()
        .unwrap_or_default();
    let repo = v["repository"]["name"].as_str().unwrap_or_default();
    let number = v["pullRequestId"].as_u64().unwrap_or(0);
    let state = match v["status"].as_str() {
        Some("completed") => ChangeRequestState::Merged,
        Some("abandoned") => ChangeRequestState::Closed,
        _ if v["isDraft"] == true => ChangeRequestState::Draft,
        _ => ChangeRequestState::Open,
    };
    let created = time(&v["creationDate"]);
    let mut cr = ChangeRequest {
        id: ChangeRequestId {
            repo: RepoRef::new(ProviderKind::Azure, host, format!("{org}/{project}/{repo}")),
            number,
        },
        title: s(&v["title"]),
        author: user(&v["createdBy"]),
        state,
        source_branch: branch(&v["sourceRefName"]),
        target_branch: branch(&v["targetRefName"]),
        reviewers: v["reviewers"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| Review {
                user: user(r),
                state: vote_state(r["vote"].as_i64().unwrap_or(0)),
            })
            .collect(),
        checks: CheckSummary::default(),
        labels: v["labels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|l| l["active"] != false)
            .filter_map(|l| l["name"].as_str().map(str::to_string))
            .collect(),
        comments_count: 0,
        additions: None,
        deletions: None,
        changed_files: None,
        // Azure has no "last updated"; closing is the only later timestamp a PR carries.
        updated_at: opt_time(&v["closedDate"])
            .filter(|c| *c > created)
            .unwrap_or(created),
        created_at: created,
        web_url: format!(
            "https://{host}/{}/{}/_git/{}/pullrequest/{number}",
            seg(org),
            seg(project),
            seg(repo)
        ),
        needs_you: false,
        signals: Signals {
            conflicts: v["mergeStatus"] == "conflicts",
            ..Default::default()
        },
    };
    // My username on this host is the unique name; match reviewers by id first.
    let me_name = cr
        .reviewers
        .iter()
        .find(|r| same(&r.user, me))
        .map(|r| r.user.username.clone())
        .unwrap_or_else(|| me.username.clone());
    cr.needs_you = compute_needs_you(&cr, &me_name);
    cr
}

/// Likes on a comment as a single thumbs-up reaction group.
fn likes(c: &Value, me: &User) -> Vec<ReactionGroup> {
    let users: Vec<User> = c["usersLiked"]
        .as_array()
        .into_iter()
        .flatten()
        .map(user)
        .collect();
    if users.is_empty() {
        return vec![];
    }
    vec![ReactionGroup {
        kind: ReactionKind::ThumbsUp,
        count: users.len() as u32,
        me: users.iter().any(|u| same(u, me)),
        users: users.iter().map(|u| u.username.clone()).collect(),
    }]
}

/// Comment ids are `thread/comment` so edits and likes can find both.
pub(crate) fn split_comment_id(id: &str) -> ProviderResult<(u64, u64)> {
    let (t, c) = id
        .split_once('/')
        .ok_or_else(|| ProviderError::Decode(format!("comment id {id} is not thread/comment")))?;
    Ok((
        t.parse().map_err(|_| ProviderError::Decode(id.into()))?,
        c.parse().map_err(|_| ProviderError::Decode(id.into()))?,
    ))
}

fn file_path(ctx: &Value) -> Option<String> {
    ctx["filePath"]
        .as_str()
        .map(|p| p.trim_start_matches('/').to_string())
        .filter(|p| !p.is_empty())
}

/// A thread's `threadContext` as a diff anchor: the right side (new) wins, else the left (old).
pub(crate) fn thread_position(ctx: &Value) -> Option<DiffPosition> {
    let path = file_path(ctx)?;
    let line = |v: &Value| v["line"].as_u64().map(|l| l as u32).filter(|l| *l > 0);
    let (side, start, end) =
        if let Some(end) = line(&ctx["rightFileEnd"]).or(line(&ctx["rightFileStart"])) {
            (DiffSide::New, line(&ctx["rightFileStart"]), end)
        } else {
            let end = line(&ctx["leftFileEnd"]).or(line(&ctx["leftFileStart"]))?;
            (DiffSide::Old, line(&ctx["leftFileStart"]), end)
        };
    Some(DiffPosition {
        path,
        side,
        line: end,
        start_line: start.filter(|s| *s < end),
        ..Default::default()
    })
}

pub(crate) fn comment(thread_id: u64, c: &Value) -> Comment {
    comment_for(thread_id, c, None, &User::default())
}

fn comment_for(thread_id: u64, c: &Value, pos: Option<&DiffPosition>, me: &User) -> Comment {
    Comment {
        id: format!("{thread_id}/{}", c["id"].as_u64().unwrap_or(0)),
        author: user(&c["author"]),
        body: s(&c["content"]),
        created_at: time(&c["publishedDate"]),
        position: pos.map(|p| InlinePosition {
            path: p.path.clone(),
            line: Some(p.line),
        }),
        reactions: likes(c, me),
        suggestions: vec![],
    }
}

/// Resolved state from a thread status; None for threads without one (system threads).
pub(crate) fn thread_resolved(status: Option<&str>) -> Option<bool> {
    match status? {
        "active" | "pending" => Some(false),
        "fixed" | "wontFix" | "closed" | "byDesign" => Some(true),
        _ => None,
    }
}

/// Discussion threads (text comments only; system and deleted ones are dropped).
pub(crate) fn parse_threads(threads: &[Value], me: &User) -> Vec<Thread> {
    threads
        .iter()
        .filter(|t| t["isDeleted"] != true)
        .filter_map(|t| {
            let tid = t["id"].as_u64()?;
            let pos = thread_position(&t["threadContext"]);
            let comments: Vec<Comment> = t["comments"]
                .as_array()?
                .iter()
                .filter(|c| {
                    c["isDeleted"] != true && c["commentType"].as_str().unwrap_or("text") == "text"
                })
                .map(|c| comment_for(tid, c, pos.as_ref(), me))
                .collect();
            if comments.is_empty() {
                return None;
            }
            Some(Thread {
                id: tid.to_string(),
                resolved: thread_resolved(t["status"].as_str()),
                comments,
                position: pos,
                resolved_by: None,
                resolved_at: None,
                outdated: false,
            })
        })
        .collect()
}

fn prop<'a>(t: &'a Value, name: &str) -> Option<&'a str> {
    t["properties"][name]["$value"].as_str()
}

/// Vote timeline events from the system "VoteUpdate" threads.
pub(crate) fn vote_events(threads: &[Value]) -> Vec<TimelineEvent> {
    threads
        .iter()
        .filter(|t| prop(t, "CodeReviewThreadType") == Some("VoteUpdate"))
        .filter_map(|t| {
            let vote: i64 = prop(t, "CodeReviewVoteResult")?.parse().ok()?;
            let state = vote_state(vote);
            if state == ReviewState::Pending {
                return None;
            }
            let first = t["comments"].as_array().and_then(|c| c.first());
            let actor = prop(t, "CodeReviewVotedByIdentity")
                .map(|k| &t["identities"][k])
                .filter(|i| i.is_object())
                .map(user)
                .or_else(|| first.map(|c| user(&c["author"])))?;
            let at = opt_time(&t["publishedDate"])
                .or_else(|| first.and_then(|c| opt_time(&c["publishedDate"])))
                .unwrap_or_default();
            Some(TimelineEvent {
                kind: TimelineKind::Reviewed(state),
                actor,
                at,
            })
        })
        .collect()
}

/// Completed or abandoned, from the PR payload.
pub(crate) fn close_event(pr: &Value) -> Option<TimelineEvent> {
    let kind = match pr["status"].as_str()? {
        "completed" => TimelineKind::Merged,
        "abandoned" => TimelineKind::Closed,
        _ => return None,
    };
    let actor = if pr["closedBy"].is_object() {
        user(&pr["closedBy"])
    } else {
        user(&pr["createdBy"])
    };
    Some(TimelineEvent {
        kind,
        actor,
        at: opt_time(&pr["closedDate"])?,
    })
}

/// `GET .../commits` is newest first; we keep oldest first.
pub(crate) fn parse_commits(items: &[Value], repo: &RepoRef) -> Vec<CommitInfo> {
    let mut out: Vec<CommitInfo> = items
        .iter()
        .map(|c| {
            let sha = s(&c["commitId"]);
            CommitInfo {
                message: c["comment"]
                    .as_str()
                    .unwrap_or_default()
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                author: c["author"]["name"].as_str().unwrap_or_default().to_string(),
                at: opt_time(&c["author"]["date"]),
                url: Some(format!("{}/commit/{sha}", repo.web_url())),
                sha,
            }
        })
        .collect();
    out.reverse();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn me() -> User {
        User {
            username: "me@example.com".into(),
            name: Some("Me".into()),
            id: Some("11111111-1111-1111-1111-111111111111".into()),
            ..Default::default()
        }
    }

    fn fixture(name: &str) -> Value {
        let text = match name {
            "pr" => include_str!("../fixtures/pull_request.json"),
            "list" => include_str!("../fixtures/pull_requests.json"),
            "threads" => include_str!("../fixtures/threads.json"),
            "commits" => include_str!("../fixtures/commits.json"),
            "connection" => include_str!("../fixtures/connection_data.json"),
            _ => unreachable!(),
        };
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn encodes_and_splits() {
        assert_eq!(seg("My Project"), "My%20Project");
        assert_eq!(seg("My%20Project"), "My%20Project");
        assert_eq!(query("/src/a b.rs"), "%2Fsrc%2Fa%20b.rs");
        assert_eq!(split_repo("org/proj/repo"), Some(("org", "proj", "repo")));
        assert_eq!(split_repo("org/proj"), None);
    }

    #[test]
    fn votes() {
        assert_eq!(vote_state(10), ReviewState::Approved);
        assert_eq!(vote_state(5), ReviewState::Approved);
        assert_eq!(vote_state(0), ReviewState::Pending);
        assert_eq!(vote_state(-5), ReviewState::ChangesRequested);
        assert_eq!(vote_state(-10), ReviewState::ChangesRequested);
        assert_eq!(vote_action(ReviewAction::Approve), Some(10));
        assert_eq!(vote_action(ReviewAction::ApproveWithSuggestions), Some(5));
        assert_eq!(vote_action(ReviewAction::ResetVote), Some(0));
        assert_eq!(vote_action(ReviewAction::Unapprove), Some(0));
        assert_eq!(vote_action(ReviewAction::WaitForAuthor), Some(-5));
        assert_eq!(vote_action(ReviewAction::RequestChanges), Some(-10));
        assert_eq!(vote_action(ReviewAction::Comment), None);
    }

    #[test]
    fn parses_pull_request() {
        let v = fixture("pr");
        let cr = parse_pr("dev.azure.com", "contoso", &v, &me());
        assert_eq!(cr.id.repo.path, "contoso/Fabrikam Web/web-app");
        assert_eq!(cr.id.number, 42);
        assert_eq!(cr.state, ChangeRequestState::Open);
        assert_eq!(
            (cr.source_branch.as_str(), cr.target_branch.as_str()),
            ("feature/login", "main")
        );
        assert_eq!(cr.author.username, "ada@example.com");
        assert_eq!(cr.labels, vec!["backend"]);
        assert_eq!(
            cr.web_url,
            "https://dev.azure.com/contoso/Fabrikam%20Web/_git/web-app/pullrequest/42"
        );
        let states: Vec<ReviewState> = cr.reviewers.iter().map(|r| r.state).collect();
        assert_eq!(
            states,
            vec![
                ReviewState::Pending,
                ReviewState::Approved,
                ReviewState::ChangesRequested
            ]
        );
        assert_eq!(
            cr.reviewers[2].user.username,
            "team:[Fabrikam Web]\\Web Team"
        );
        assert_eq!(cr.reviewers[2].user.label(), "Web Team");
        // I am a reviewer without a vote on an open PR.
        assert!(cr.needs_you);
        assert_eq!(my_vote(&v, &me()), 0);
        let list = fixture("list");
        let items: Vec<ChangeRequest> = list["value"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| parse_pr("dev.azure.com", "contoso", p, &me()))
            .collect();
        assert_eq!(items[0].state, ChangeRequestState::Draft);
        // A draft still needs me when my review is requested on it.
        assert_eq!(
            needs_reason(&items[0], &me().username).is_some(),
            items[0]
                .reviewers
                .iter()
                .any(|r| r.state == ReviewState::Pending && same(&r.user, &me()))
        );
        assert_eq!(items[1].state, ChangeRequestState::Merged);
        assert!(items[1].updated_at > items[1].created_at);
        assert_eq!(items[2].state, ChangeRequestState::Closed);
        assert_eq!(
            close_event(&list["value"][1]).unwrap().kind,
            TimelineKind::Merged
        );
        assert!(close_event(&v).is_none());
    }

    #[test]
    fn parses_threads_and_events() {
        let t = fixture("threads");
        let threads = parse_threads(t["value"].as_array().unwrap(), &me());
        assert_eq!(threads.len(), 2, "system and deleted threads are dropped");
        let general = &threads[0];
        assert_eq!(
            (
                general.id.as_str(),
                general.resolved,
                general.position.is_none()
            ),
            ("1", Some(false), true)
        );
        assert_eq!(general.comments.len(), 2);
        assert_eq!(general.comments[0].id, "1/1");
        let g = &general.comments[0].reactions[0];
        assert_eq!((g.kind, g.count, g.me), (ReactionKind::ThumbsUp, 2, true));
        let inline = &threads[1];
        assert_eq!(inline.resolved, Some(true));
        let p = inline.position.as_ref().unwrap();
        assert_eq!(
            (p.path.as_str(), p.side, p.line, p.start_line),
            ("src/app.ts", DiffSide::New, 14, Some(12))
        );
        assert_eq!(inline.comments[0].position.as_ref().unwrap().line, Some(14));
        let ev = vote_events(t["value"].as_array().unwrap());
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].kind, TimelineKind::Reviewed(ReviewState::Approved));
        assert_eq!(ev[0].actor.username, "bob@example.com");
        assert_eq!(split_comment_id("7/3").unwrap(), (7, 3));
        assert!(split_comment_id("7").is_err());
    }

    #[test]
    fn left_side_anchor() {
        let p = thread_position(&json!({"filePath": "/old.rs", "leftFileStart": {"line": 4, "offset": 1}, "leftFileEnd": {"line": 4, "offset": 9}})).unwrap();
        assert_eq!((p.side, p.line, p.start_line), (DiffSide::Old, 4, None));
        assert!(thread_position(&json!({"filePath": "/x"})).is_none());
        assert_eq!(thread_resolved(Some("wontFix")), Some(true));
        assert_eq!(thread_resolved(None), None);
    }

    #[test]
    fn users_and_commits() {
        let u = connection_user(&fixture("connection")).unwrap();
        assert_eq!(
            (u.username.as_str(), u.id.as_deref()),
            (
                "me@example.com",
                Some("11111111-1111-1111-1111-111111111111")
            )
        );
        let p = profile_user(
            &json!({"id": "abc", "displayName": "Me", "emailAddress": "me@example.com"}),
        )
        .unwrap();
        assert_eq!(p.username, "me@example.com");
        let i = identity_user(&json!({"id": "x", "providerDisplayName": "Ada", "properties": {"Mail": {"$value": "ada@example.com"}}})).unwrap();
        assert_eq!(
            (i.username.as_str(), i.id.as_deref()),
            ("ada@example.com", Some("x"))
        );
        assert!(identity_user(&json!({"id": "g", "isContainer": true})).is_none());
        let bot = user(
            &json!({"displayName": "Fabrikam Build Service (contoso)", "uniqueName": "Build\\abc", "id": "b"}),
        );
        assert!(bot.bot);
        let named = user(
            &json!({"displayName": "Ada Lovelace <ada@example.com>", "uniqueName": "ada@example.com", "imageUrl": "https://dev.azure.com/o/_api/_common/identityImage?id=1"}),
        );
        assert_eq!(named.username, "ada@example.com");
        assert_eq!(named.name.as_deref(), Some("Ada Lovelace"));
        assert!(named.avatar_url.is_some());
        let team = user(
            &json!({"displayName": "[Proj]\\Core Team", "uniqueName": "vstfs:///Classification/TeamProject/x", "isContainer": true, "imageUrl": "https://dev.azure.com/o/i"}),
        );
        assert_eq!(team.username, "team:[Proj]\\Core Team");
        assert_eq!(team.name.as_deref(), Some("Core Team"));
        assert!(team.avatar_url.is_none());
        let repo = RepoRef::new(ProviderKind::Azure, "dev.azure.com", "contoso/proj/repo");
        let c = parse_commits(fixture("commits")["value"].as_array().unwrap(), &repo);
        assert_eq!(c[0].message, "Add login form");
        assert_eq!(c[1].url.as_deref(), Some("https://dev.azure.com/contoso/proj/_git/repo/commit/bbbb000000000000000000000000000000000002"));
    }
}
