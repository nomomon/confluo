//! Pending text posts (comments, replies, inline comments, edits). Each has a client id and lives
//! here, apart from the server detail, until the server copy shows up in a refetch; `apply` lays
//! them over freshly fetched threads, so a refresh never drops a pending or failed item.

use chrono::{DateTime, Duration, Utc};
use gv_engine::{find_post, Comment, Post, Thread, User};

/// Comment ids of items not yet on the server start with this; they are never sent anywhere.
pub const PREFIX: &str = "pending:";
/// A sent item whose server copy never shows up is dropped after this.
const KEEP_SENT: i64 = 10 * 60;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Sending,
    /// Reason, shown on hover.
    Failed(String),
    /// The server copy from the POST response, shown until a refetch has it.
    Sent(Box<Comment>),
}

#[derive(Clone, Debug)]
pub struct Pending {
    pub cid: String,
    pub post: Post,
    pub body: String,
    pub author: User,
    pub at: DateTime<Utc>,
    pub status: Status,
    /// Edit: the body before, restored on Discard.
    pub before: Option<String>,
}

impl Pending {
    /// The id this item renders under.
    pub fn comment_id(&self) -> String {
        match (&self.status, &self.post) {
            (Status::Sent(c), _) => c.id.clone(),
            (_, Post::Edit(id)) => id.clone(),
            _ => format!("{PREFIX}{}", self.cid),
        }
    }

    fn comment(&self) -> Comment {
        match &self.status {
            Status::Sent(c) => (**c).clone(),
            _ => Comment {
                id: self.comment_id(),
                author: self.author.clone(),
                body: self.body.clone(),
                created_at: self.at,
                position: None,
                reactions: vec![],
                suggestions: vec![],
            },
        }
    }
}

#[derive(Default, Debug)]
pub struct Outbox {
    pub items: Vec<Pending>,
    next: u64,
}

impl Outbox {
    /// Queue a new item as Sending; returns its client id.
    pub fn push(
        &mut self,
        post: Post,
        body: String,
        author: User,
        before: Option<String>,
    ) -> String {
        self.next += 1;
        let cid = self.next.to_string();
        self.items.push(Pending {
            cid: cid.clone(),
            post,
            body,
            author,
            at: Utc::now(),
            status: Status::Sending,
            before,
        });
        cid
    }

    pub fn get(&self, cid: &str) -> Option<&Pending> {
        self.items.iter().find(|p| p.cid == cid)
    }

    pub fn set(&mut self, cid: &str, s: Status) {
        if let Some(p) = self.items.iter_mut().find(|p| p.cid == cid) {
            p.status = s;
        }
    }

    pub fn remove(&mut self, cid: &str) -> Option<Pending> {
        let i = self.items.iter().position(|p| p.cid == cid)?;
        Some(self.items.remove(i))
    }

    /// Something sending blocks a second submit of the same text (double submit guard).
    pub fn sending(&self, body: &str) -> bool {
        self.items
            .iter()
            .any(|p| p.status == Status::Sending && p.body.trim() == body.trim())
    }

    /// Status of the item rendered as comment `id`, if it is one of ours and not yet confirmed.
    #[cfg(test)]
    pub fn status_of(&self, id: &str) -> Option<&Status> {
        self.pending_for(id).map(|p| &p.status)
    }

    /// The unconfirmed item rendered as comment `id`.
    pub fn pending_for(&self, id: &str) -> Option<&Pending> {
        self.items
            .iter()
            .find(|p| !matches!(p.status, Status::Sent(_)) && p.comment_id() == id)
    }

    /// Drop items whose server copy is in `threads` (by id, or my comment with the same body).
    pub fn reconcile(&mut self, threads: &[Thread], me: Option<&str>) {
        let now = Utc::now();
        self.items.retain(|p| {
            let found = match &p.status {
                Status::Sent(c) if !matches!(p.post, Post::Edit(_)) => threads
                    .iter()
                    .any(|t| t.comments.iter().any(|x| x.id == c.id)),
                _ => find_post(threads, &p.post, &p.body, me, p.at).is_some(),
            };
            let stale =
                matches!(p.status, Status::Sent(_)) && now - p.at > Duration::seconds(KEEP_SENT);
            !found && !stale
        });
    }

    /// Lay the items over server `threads` (idempotent).
    pub fn apply(&self, threads: &mut Vec<Thread>) {
        for t in threads.iter_mut() {
            t.comments.retain(|c| !c.id.starts_with(PREFIX));
        }
        threads.retain(|t| !(t.id.starts_with(PREFIX) && t.comments.is_empty()));
        for p in &self.items {
            let c = p.comment();
            let id = c.id.clone();
            let has = |ts: &Vec<Thread>| ts.iter().any(|t| t.comments.iter().any(|x| x.id == id));
            match &p.post {
                Post::Edit(cid) => threads
                    .iter_mut()
                    .flat_map(|t| t.comments.iter_mut())
                    .filter(|x| &x.id == cid)
                    .for_each(|x| x.body = c.body.clone()),
                _ if has(threads) => {}
                Post::Reply(tid) if threads.iter().any(|t| &t.id == tid) => {
                    if let Some(t) = threads.iter_mut().find(|t| &t.id == tid) {
                        t.comments.push(c);
                    }
                }
                post => {
                    let mut t = Thread::conversation(format!("{PREFIX}{}", p.cid), vec![c]);
                    if let Post::Line(pos) = post {
                        t.position = Some(pos.clone());
                        t.resolved = Some(false);
                    }
                    threads.push(t);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "outbox_tests.rs"]
mod tests;
