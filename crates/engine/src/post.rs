//! Posting text (comment, reply, inline comment, edit) with one automatic retry on transient
//! failures and a duplicate guard: after a network error the request may have landed, so the
//! fresh detail is checked for a matching comment before sending again.

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

use crate::{ChangeRequestId, Comment, DiffPosition, Engine, ProviderError, Thread};

/// What a text post does.
#[derive(Clone, Debug, PartialEq)]
pub enum Post {
    Comment,
    Reply(String),
    Line(DiffPosition),
    /// Edit of the comment with this id.
    Edit(String),
}

/// Clock skew allowed between us and the host when matching a posted comment.
const SKEW: i64 = 120;

/// Whether `c` is the server copy of `body` posted by `me` at `at` (or later).
pub fn same_post(c: &Comment, body: &str, me: Option<&str>, at: DateTime<Utc>) -> bool {
    c.body.trim() == body.trim()
        && me.is_some_and(|m| m.eq_ignore_ascii_case(&c.author.username))
        && c.created_at >= at - Duration::seconds(SKEW)
}

/// The server comment matching post `p` of `body`, if one is in `threads`.
pub fn find_post<'a>(
    threads: &'a [Thread],
    p: &Post,
    body: &str,
    me: Option<&str>,
    at: DateTime<Utc>,
) -> Option<&'a Comment> {
    let mut all = threads.iter().flat_map(|t| t.comments.iter());
    match p {
        Post::Edit(cid) => all.find(|c| &c.id == cid && c.body.trim() == body.trim()),
        _ => all.find(|c| same_post(c, body, me, at)),
    }
}

/// Worth one automatic retry: network errors and server errors (5xx).
pub fn transient(e: &anyhow::Error) -> bool {
    matches!(
        e.downcast_ref::<ProviderError>(),
        Some(ProviderError::Network(_))
            | Some(ProviderError::Http {
                status: 500..=599,
                ..
            })
    )
}

impl Engine {
    /// Send post `p`. Retries once on a transient error; after a network error it first looks for
    /// the comment on the host so a request that did land is not posted twice. Auth refresh on 401
    /// happens in the providers.
    pub async fn post(
        &self,
        id: &ChangeRequestId,
        p: &Post,
        body: &str,
        at: DateTime<Utc>,
    ) -> Result<Comment> {
        match self.post_once(id, p, body).await {
            Err(e) if transient(&e) => {
                if let Some(c) = self.posted(id, p, body, at).await {
                    return Ok(c);
                }
                self.post_once(id, p, body).await
            }
            r => r,
        }
    }

    /// Look for post `p` in a fresh detail (duplicate guard).
    pub async fn posted(
        &self,
        id: &ChangeRequestId,
        p: &Post,
        body: &str,
        at: DateTime<Utc>,
    ) -> Option<Comment> {
        self.forget_detail(id);
        let d = self.detail(id, false).await.ok()?;
        let me = self.me(id.repo.kind, &id.repo.host);
        find_post(&d.threads, p, body, me.as_deref(), at).cloned()
    }

    async fn post_once(&self, id: &ChangeRequestId, p: &Post, body: &str) -> Result<Comment> {
        match p {
            Post::Comment => self.add_comment(id, body).await,
            Post::Reply(t) => self.reply_to_thread(id, t, body).await,
            Post::Line(pos) => self.add_line_comment(id, pos, body).await,
            Post::Edit(c) => self.edit_comment(id, c, body).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::User;

    fn c(id: &str, who: &str, body: &str, at: DateTime<Utc>) -> Comment {
        Comment {
            id: id.into(),
            author: User::named(who),
            body: body.into(),
            created_at: at,
            position: None,
            reactions: vec![],
            suggestions: vec![],
        }
    }

    #[test]
    fn finds_my_matching_post_only() {
        let t0 = Utc::now();
        let ts = vec![Thread::conversation(
            "t",
            vec![
                c("1", "ada", "hi", t0),
                c("2", "me", "hi", t0 - Duration::hours(1)),
                c("3", "me", "hi ", t0 + Duration::seconds(3)),
            ],
        )];
        let hit = find_post(&ts, &Post::Comment, "hi", Some("Me"), t0).unwrap();
        assert_eq!(hit.id, "3");
        assert!(find_post(&ts, &Post::Comment, "other", Some("me"), t0).is_none());
        assert!(find_post(&ts, &Post::Edit("2".into()), "hi", Some("me"), t0).is_some());
    }

    #[test]
    fn only_network_and_server_errors_are_transient() {
        let t = |e: ProviderError| transient(&anyhow::Error::new(e));
        assert!(t(ProviderError::Network("reset".into())));
        assert!(t(ProviderError::Http {
            status: 502,
            body: String::new()
        }));
        assert!(!t(ProviderError::NotFound("x".into())));
        assert!(!t(ProviderError::Http {
            status: 422,
            body: String::new()
        }));
    }
}
