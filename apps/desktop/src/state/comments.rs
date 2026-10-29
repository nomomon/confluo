//! Posting, replying to and editing comments with optimistic updates.

use chrono::Utc;
use gpui::Context;
use gv_engine::{Comment, Thread, User};

use super::{bridge, err_str, DetailTab};
use crate::app::GitViewer;

impl GitViewer {
    pub(crate) fn post_comment(&mut self, cx: &mut Context<Self>) {
        let body = self.composer.read(cx).text().trim().to_string();
        let Some(d) = self.detail.as_mut() else { return };
        if body.is_empty() || d.posting {
            return;
        }
        let id = d.summary.id.clone();
        let me = User::named(d.me.clone().unwrap_or_else(|| "you".into()));
        let optimistic = Comment { id: "pending".into(), author: me, body: body.clone(), created_at: Utc::now(), position: None };
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
                det.threads.push(Thread { id: "pending".into(), resolved: None, comments: vec![optimistic] });
            }
        }
        d.posting = true;
        d.tab = DetailTab::Activity;
        self.composer.update(cx, |i, cx| i.set_text("", cx));
        let engine = self.engine.clone();
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
                    }
                }
                v.load_detail(cx);
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
