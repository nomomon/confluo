//! Posting, replying to and editing comments (through the outbox), and deleting them.

use gpui::Context;
use gv_engine::{Post, Thread};

use crate::app::GitViewer;

impl GitViewer {
    /// Send the composer text as a comment, reply or edit through the outbox (`state::outbox`).
    pub(crate) fn post_comment(&mut self, cx: &mut Context<Self>) {
        let body = self.composer.read(cx).text().trim().to_string();
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        if body.is_empty() {
            return;
        }
        let (reply_to, editing) = (d.reply_to.clone(), d.editing.clone());
        let before = editing.as_ref().and_then(|cid| {
            d.detail.as_ref().and_then(|det| {
                det.threads
                    .iter()
                    .flat_map(|t| t.comments.iter())
                    .find(|c| &c.id == cid)
                    .map(|c| c.body.clone())
            })
        });
        let post = match (editing, reply_to) {
            (Some(c), _) => Post::Edit(c),
            (None, Some(t)) => Post::Reply(t),
            (None, None) => Post::Comment,
        };
        if !self.send_post(post, body, before, cx) {
            return;
        }
        if let Some(d) = self.detail.as_mut() {
            d.reply_to = None;
            d.editing = None;
        }
        self.review.right_tab = super::RightTab::Conversation;
        self.review.right_open = true;
        self.composer.update(cx, |i, cx| i.set_text("", cx));
        cx.notify();
    }

    /// Ask before deleting comment `cid` (own comments only; the menu offers it only for those).
    pub(crate) fn confirm_delete_comment(&mut self, cid: String, cx: &mut Context<Self>) {
        if let Some(d) = self.detail.as_mut() {
            d.ui.delete_comment = Some(cid);
            self.layer = None;
            self.overlay = crate::app::Overlay::ConfirmDeleteComment;
        }
        cx.notify();
    }

    /// Delete the confirmed comment: gone at once, back with a toast if the provider refuses.
    pub(crate) fn delete_comment(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        let Some(cid) = d.ui.delete_comment.take() else {
            return;
        };
        self.overlay = crate::app::Overlay::None;
        self.refocus = true;
        let (engine, id, c2) = (self.engine.clone(), d.summary.id.clone(), cid.clone());
        self.run_edit(
            "Comment deleted",
            "Deleting the comment",
            move |d| {
                if let Some(det) = d.detail.as_mut() {
                    remove_comment(&mut det.threads, &c2);
                }
            },
            async move { engine.delete_comment(&id, &cid).await },
            cx,
        );
    }
}

/// Drop comment `cid`; a thread left without comments goes too (an inline thread's last comment).
pub(crate) fn remove_comment(threads: &mut Vec<Thread>, cid: &str) {
    for t in threads.iter_mut() {
        t.comments.retain(|c| c.id != cid);
    }
    threads.retain(|t| !t.comments.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use gv_engine::{Comment, User};

    fn c(id: &str) -> Comment {
        Comment {
            id: id.into(),
            author: User::named("me"),
            body: String::new(),
            created_at: Utc::now(),
            position: None,
            suggestions: vec![],
            reactions: vec![],
        }
    }

    #[test]
    fn deleting_the_last_comment_removes_the_thread() {
        let mut ts = vec![
            Thread::conversation("a", vec![c("1"), c("2")]),
            Thread::conversation("b", vec![c("3")]),
        ];
        remove_comment(&mut ts, "2");
        assert_eq!(ts.len(), 2);
        remove_comment(&mut ts, "3");
        assert_eq!(
            ts.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            vec!["a"]
        );
    }
}
