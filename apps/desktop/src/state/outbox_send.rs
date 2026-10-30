//! Sending outbox items (`state::outbox`): render at once, replace with the server copy on success,
//! keep a failed item visible with Retry, Edit and Discard.

use gpui::{Context, Focusable, Window};
use gv_engine::{ChangeRequestId, Post, User};

use super::outbox::{Outbox, Status};
use super::{bridge, err_str};
use crate::app::GitViewer;

impl GitViewer {
    fn outbox_mut(&mut self, id: &ChangeRequestId) -> &mut Outbox {
        self.outbox.entry(id.clone()).or_default()
    }

    /// Lay the open PR's outbox over its detail; `fetched` also drops items the server now has.
    pub(crate) fn relay_outbox(&mut self, fetched: bool) {
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        if crate::dev::fake_outbox() && !self.outbox.contains_key(&d.summary.id) {
            self.outbox
                .insert(d.summary.id.clone(), fake(d.detail.as_ref()));
        }
        let Some(o) = self.outbox.get_mut(&d.summary.id) else {
            return;
        };
        if let Some(det) = d.detail.as_mut() {
            if fetched {
                o.reconcile(&det.threads, d.me.as_deref());
            }
            o.apply(&mut det.threads);
        }
        d.reindex();
    }

    /// Queue `post` of `body` on the open PR and send it. Returns false for a double submit.
    pub(crate) fn send_post(
        &mut self,
        post: Post,
        body: String,
        before: Option<String>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(d) = self.detail.as_ref() else {
            return false;
        };
        let id = d.summary.id.clone();
        let me = User::named(d.me.clone().unwrap_or_else(|| "you".into()));
        if self.outbox_mut(&id).sending(&body) {
            return false;
        }
        let cid = self.outbox_mut(&id).push(post, body, me, before);
        self.relay_outbox(false);
        self.dispatch_post(id, cid, cx);
        true
    }

    fn dispatch_post(&mut self, id: ChangeRequestId, cid: String, cx: &mut Context<Self>) {
        let Some(p) = self.outbox.get(&id).and_then(|o| o.get(&cid)).cloned() else {
            return;
        };
        let engine = self.engine.clone();
        let id2 = id.clone();
        let rx = bridge::spawn(async move {
            let r = engine.post(&id2, &p.post, &p.body, p.at).await;
            r.map_err(err_str)
        });
        cx.spawn(async move |this, cx| {
            let r = rx.await.unwrap_or_else(|_| Err("cancelled".into()));
            this.update(cx, |v, cx| {
                let ok = r.is_ok();
                let s = match r {
                    Ok(c) => Status::Sent(Box::new(c)),
                    Err(e) => Status::Failed(e),
                };
                v.outbox_mut(&id).set(&cid, s);
                v.relay_outbox(false);
                // The engine dropped its cached copy; refetch in the background to confirm.
                if ok && v.detail.as_ref().is_some_and(|d| d.summary.id == id) {
                    v.load_detail(cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn retry_post(&mut self, cid: String, cx: &mut Context<Self>) {
        let Some(id) = self.detail.as_ref().map(|d| d.summary.id.clone()) else {
            return;
        };
        self.outbox_mut(&id).set(&cid, Status::Sending);
        self.relay_outbox(false);
        self.dispatch_post(id, cid, cx);
    }

    /// Drop a failed item; an edit gets its old body back.
    pub(crate) fn discard_post(&mut self, cid: String, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        let Some(p) = self
            .outbox
            .get_mut(&d.summary.id)
            .and_then(|o| o.remove(&cid))
        else {
            return;
        };
        if let (Post::Edit(c), Some(old), Some(det)) = (&p.post, &p.before, d.detail.as_mut()) {
            for x in det.threads.iter_mut().flat_map(|t| t.comments.iter_mut()) {
                if &x.id == c {
                    x.body = old.clone();
                }
            }
        }
        self.relay_outbox(false);
        cx.notify();
    }

    /// Put a failed item's text back into a composer and drop the item.
    pub(crate) fn edit_post(&mut self, cid: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.detail.as_ref().map(|d| d.summary.id.clone()) else {
            return;
        };
        let Some(p) = self.outbox.get(&id).and_then(|o| o.get(&cid)).cloned() else {
            return;
        };
        self.discard_post(cid, cx);
        if let Some(d) = self.detail.as_mut() {
            d.reply_to = None;
            d.editing = None;
            match &p.post {
                Post::Reply(t) => d.reply_to = Some(t.clone()),
                Post::Edit(c) => d.editing = Some(c.clone()),
                _ => {}
            }
        }
        // An inline comment's anchor cannot be reopened from here; its text goes to the composer.
        self.composer.update(cx, |i, cx| i.set_text(p.body, cx));
        window.focus(&self.composer.focus_handle(cx));
        cx.notify();
    }
}

/// Dev: one comment being sent and one failed reply (on the first thread).
fn fake(det: Option<&gv_engine::ChangeRequestDetail>) -> Outbox {
    let mut o = Outbox::default();
    let me = User::named("you");
    o.push(
        Post::Comment,
        "Looks good, merging after CI.".into(),
        me.clone(),
        None,
    );
    let thread = det.and_then(|d| d.threads.first()).map(|t| t.id.clone());
    let post = thread.map(Post::Reply).unwrap_or(Post::Comment);
    let cid = o.push(
        post,
        "Can we keep the old flag for one release?".into(),
        me,
        None,
    );
    o.set(&cid, Status::Failed("GitLab: not found (404)".into()));
    o
}
