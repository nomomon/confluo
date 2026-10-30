//! Which review threads need the viewer: the attention state of each thread.

use crate::{ChangeRequestDetail, Thread};

/// Attention state of a thread, most urgent first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Attention {
    /// Unresolved, and I am mentioned or someone else spoke last on a thread I am part of.
    NeedsMe,
    /// Unresolved, waiting on others (my reply was last, or I am not involved).
    Open,
    /// Unresolved but anchored on a line no longer in the diff.
    Outdated,
    Resolved,
    /// Not resolvable (plain conversation comments).
    Plain,
}

impl Attention {
    pub fn unresolved(self) -> bool {
        matches!(self, Self::NeedsMe | Self::Open | Self::Outdated)
    }
}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// `@me` as a whole word in `body`.
fn mentions(body: &str, me: &str) -> bool {
    let needle = format!("@{}", me.to_lowercase());
    let lower = body.to_lowercase();
    lower.match_indices(&needle).any(|(i, _)| {
        let end = i + needle.len();
        !lower[end..].chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
    })
}

/// Classify one thread for `me`. `involved` says I am the PR author or a reviewer; writing in the
/// thread also makes me a participant. GitHub `isResolved`, GitLab `resolvable`/`resolved` and Azure
/// thread status all arrive as `Thread::resolved` (None when the thread cannot be resolved).
pub fn thread_attention(t: &Thread, me: Option<&str>, involved: bool) -> Attention {
    match t.resolved {
        None => return Attention::Plain,
        Some(true) => return Attention::Resolved,
        Some(false) => {}
    }
    if t.outdated {
        return Attention::Outdated;
    }
    let Some(me) = me.filter(|m| !m.is_empty()) else { return Attention::Open };
    let participant = t.comments.iter().any(|c| same(&c.author.username, me));
    let mentioned = t.comments.iter().any(|c| !same(&c.author.username, me) && mentions(&c.body, me));
    let last_not_mine = t.comments.last().is_some_and(|c| !same(&c.author.username, me));
    if mentioned || (last_not_mine && (involved || participant)) {
        Attention::NeedsMe
    } else {
        Attention::Open
    }
}

/// Every thread's attention, in thread order. I am involved when I authored or review the PR.
pub fn threads_attention(det: &ChangeRequestDetail, me: Option<&str>) -> Vec<Attention> {
    let involved = me.is_some_and(|m| same(&det.summary.author.username, m) || det.summary.reviewers.iter().any(|r| same(&r.user.username, m)));
    det.threads.iter().map(|t| thread_attention(t, me, involved)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Comment, User};
    use chrono::Utc;

    fn c(who: &str, body: &str) -> Comment {
        Comment { id: who.into(), author: User { username: who.into(), ..Default::default() }, body: body.into(), created_at: Utc::now(), position: None, reactions: vec![] }
    }

    fn t(resolved: Option<bool>, outdated: bool, comments: Vec<Comment>) -> Thread {
        Thread { id: "t".into(), resolved, comments, position: None, outdated }
    }

    #[test]
    fn classifies_threads() {
        let me = Some("ada");
        // Not resolvable, resolved, outdated.
        assert_eq!(thread_attention(&t(None, false, vec![c("bob", "hi")]), me, true), Attention::Plain);
        assert_eq!(thread_attention(&t(Some(true), false, vec![c("bob", "@ada fix")]), me, true), Attention::Resolved);
        assert_eq!(thread_attention(&t(Some(false), true, vec![c("bob", "x")]), me, true), Attention::Outdated);
        // Someone else spoke last on my PR: needs me. My reply was last: waiting on others.
        assert_eq!(thread_attention(&t(Some(false), false, vec![c("bob", "why?")]), me, true), Attention::NeedsMe);
        assert_eq!(thread_attention(&t(Some(false), false, vec![c("bob", "why?"), c("ADA", "because")]), me, true), Attention::Open);
        // Not involved and not a participant: open; mentioned: needs me; participated: needs me.
        assert_eq!(thread_attention(&t(Some(false), false, vec![c("bob", "x"), c("cy", "y")]), me, false), Attention::Open);
        assert_eq!(thread_attention(&t(Some(false), false, vec![c("bob", "cc @Ada.")]), me, false), Attention::NeedsMe);
        assert_eq!(thread_attention(&t(Some(false), false, vec![c("bob", "cc @adam")]), me, false), Attention::Open);
        assert_eq!(thread_attention(&t(Some(false), false, vec![c("ada", "q"), c("bob", "a")]), me, false), Attention::NeedsMe);
        // Unknown user: never "needs me".
        assert_eq!(thread_attention(&t(Some(false), false, vec![c("bob", "x")]), None, true), Attention::Open);
        assert!(Attention::Outdated.unresolved() && !Attention::Resolved.unresolved());
    }
}
