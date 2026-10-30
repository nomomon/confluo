//! Posting, replying to and editing comments with optimistic updates.

use chrono::Utc;
use gpui::Context;
use gv_engine::{Comment, Thread, User};

use super::{bridge, err_str};
use crate::app::GitViewer;

impl GitViewer {
    pub(crate) fn post_comment(&mut self, cx: &mut Context<Self>) {
        let body = self.composer.read(cx).text().trim().to_string();
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        if body.is_empty() || d.posting {
            return;
        }
        let id = d.summary.id.clone();
        let me = User::named(d.me.clone().unwrap_or_else(|| "you".into()));
        let optimistic = Comment {
            id: "pending".into(),
            author: me,
            body: body.clone(),
            created_at: Utc::now(),
            position: None,
            reactions: vec![],
        };
        let reply_to = d.reply_to.take();
        let editing = d.editing.take();
        if let Some(det) = d.detail.as_mut() {
            if let Some(cid) = &editing {
                for c in det.threads.iter_mut().flat_map(|t| t.comments.iter_mut()) {
                    if &c.id == cid {
                        c.body = body.clone();
                    }
                }
            } else if let Some(tid) = &reply_to {
                if let Some(t) = det.threads.iter_mut().find(|t| &t.id == tid) {
                    t.comments.push(optimistic);
                }
            } else {
                det.threads
                    .push(Thread::conversation("pending", vec![optimistic]));
            }
        }
        d.posting = true;
        self.review.right_tab = super::RightTab::Conversation;
        self.review.right_open = true;
        self.composer.update(cx, |i, cx| i.set_text("", cx));
        let engine = self.engine.clone();
        let (restore_reply, restore_edit, restore_body) =
            (reply_to.clone(), editing.clone(), body.clone());
        let rx = bridge::spawn(async move {
            let r = if let Some(cid) = editing {
                engine.edit_comment(&id, &cid, &body).await
            } else if let Some(tid) = reply_to {
                engine.reply_to_thread(&id, &tid, &body).await
            } else {
                engine.add_comment(&id, &body).await
            };
            r.map(|_| ()).map_err(err_str)
        });
        cx.spawn(async move |this, cx| {
            let r = rx.await.unwrap_or_else(|_| Err("cancelled".into()));
            this.update(cx, |v, cx| {
                if let Some(d) = v.detail.as_mut() {
                    d.posting = false;
                    if let Err(e) = r {
                        d.error = Some(format!("Posting failed: {e}"));
                        d.reply_to = restore_reply;
                        d.editing = restore_edit;
                        if v.composer.read(cx).text().trim().is_empty() {
                            v.composer.update(cx, |i, cx| i.set_text(restore_body, cx));
                        }
                    }
                }
                v.load_detail(cx);
            })
            .ok();
        })
        .detach();
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

    fn c(id: &str) -> Comment {
        Comment {
            id: id.into(),
            author: User::named("me"),
            body: String::new(),
            created_at: Utc::now(),
            position: None,
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
