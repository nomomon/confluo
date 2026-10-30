//! Inline comment actions on the Code tab: open a composer on a line or range, post, reply, edit, resolve.

use chrono::Utc;
use gpui::{Context, Focusable, Window};
use gv_engine::{Comment, Thread, User};

use super::line_select::comment_span;
use super::{bridge, err_str, DetailTab, LineDraft};
use crate::app::GitViewer;

/// What the shared inline input submits.
enum Mode {
    New(LineDraft),
    Reply(String),
    Edit(String),
}

impl GitViewer {
    /// Open the composer on lines `lo..=last` of one hunk, anchored on the last line's side.
    pub(crate) fn open_line_draft(&mut self, fi: usize, hi: usize, lo: usize, last: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        let Some(files) = d.code.files.clone() else { return };
        let Some(f) = files.get(fi) else { return };
        let Some(lines) = f.hunks.get(hi).and_then(|h| h.lines.get(lo..=last)) else { return };
        let Some((side, start, line, other_line)) = comment_span(lines) else { return };
        let inline = &mut d.code.inline;
        let keep_text = inline.draft.is_some();
        inline.clear_modes();
        let path = match side {
            gv_engine::DiffSide::Old => f.old_path.clone(),
            gv_engine::DiffSide::New => f.new_path.clone(),
        };
        let old_path = (f.old_path != f.new_path).then(|| f.old_path.clone());
        inline.draft = Some(LineDraft { fi, path, old_path, side, line, start, other_line });
        if !keep_text {
            self.inline_input.update(cx, |i, cx| i.set_text("", cx));
        }
        d.code.sel = None;
        d.code.sync_rows();
        window.focus(&self.inline_input.focus_handle(cx));
        cx.notify();
    }

    /// Open the reply field of a thread, or the edit field of a comment when `edit` holds its body.
    pub(crate) fn open_inline_field(&mut self, thread_id: Option<String>, edit: Option<(String, String)>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.code.inline.clear_modes();
        d.code.inline.reply_to = thread_id;
        let text = edit.as_ref().map(|(_, b)| b.clone()).unwrap_or_default();
        d.code.inline.editing = edit.map(|(id, _)| id);
        d.code.sync_rows();
        self.inline_input.update(cx, |i, cx| i.set_text(text, cx));
        window.focus(&self.inline_input.focus_handle(cx));
        cx.notify();
    }

    /// Close the composer or reply/edit field. Returns whether anything was open.
    pub(crate) fn cancel_inline(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(d) = self.detail.as_mut() else { return false };
        let i = &mut d.code.inline;
        let open = i.draft.is_some() || i.reply_to.is_some() || i.editing.is_some();
        i.clear_modes();
        d.code.sync_rows();
        self.inline_input.update(cx, |i, cx| i.set_text("", cx));
        cx.notify();
        open
    }

    pub(crate) fn submit_inline(&mut self, cx: &mut Context<Self>) {
        let body = self.inline_input.read(cx).text().trim().to_string();
        let Some(d) = self.detail.as_mut() else { return };
        let inline = &mut d.code.inline;
        if body.is_empty() || inline.posting {
            return;
        }
        let mode = if let Some(c) = inline.editing.take() {
            Mode::Edit(c)
        } else if let Some(dr) = inline.draft.take() {
            Mode::New(dr)
        } else if let Some(t) = inline.reply_to.take() {
            Mode::Reply(t)
        } else {
            return;
        };
        inline.clear_modes();
        inline.posting = true;
        let me = User::named(d.me.clone().unwrap_or_else(|| "you".into()));
        let pending = Comment { id: "pending".into(), author: me, body: body.clone(), created_at: Utc::now(), position: None, reactions: vec![] };
        if let Some(det) = d.detail.as_mut() {
            match &mode {
                Mode::Edit(cid) => det.threads.iter_mut().flat_map(|t| t.comments.iter_mut()).filter(|c| &c.id == cid).for_each(|c| c.body = body.clone()),
                Mode::Reply(tid) => det.threads.iter_mut().filter(|t| &t.id == tid).for_each(|t| t.comments.push(pending.clone())),
                Mode::New(dr) => {
                    let mut t = Thread::conversation("pending", vec![pending]);
                    t.position = Some(dr.position());
                    t.resolved = Some(false);
                    det.threads.push(t);
                }
            }
        }
        d.reindex();
        self.inline_input.update(cx, |i, cx| i.set_text("", cx));
        let id = d.summary.id.clone();
        let engine = self.engine.clone();
        let rx = bridge::spawn(async move {
            let r = match mode {
                Mode::Edit(cid) => engine.edit_comment(&id, &cid, &body).await,
                Mode::Reply(tid) => engine.reply_to_thread(&id, &tid, &body).await,
                Mode::New(dr) => engine.add_line_comment(&id, &dr.position(), &body).await,
            };
            r.map(|_| ()).map_err(err_str)
        });
        self.after_inline(rx, "Comment failed", cx);
    }

    pub(crate) fn set_thread_resolved(&mut self, thread_id: String, resolved: bool, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        if let Some(t) = d.detail.as_mut().and_then(|det| det.threads.iter_mut().find(|t| t.id == thread_id)) {
            t.resolved = Some(resolved);
        }
        d.code.inline.expanded.remove(&thread_id);
        let id = d.summary.id.clone();
        let engine = self.engine.clone();
        let rx = bridge::spawn(async move { engine.resolve_thread(&id, &thread_id, resolved).await.map_err(err_str) });
        self.after_inline(rx, "Resolve failed", cx);
    }

    /// "Show in code" from the Activity tab: switch tabs and scroll to the thread once rows exist.
    pub(crate) fn show_thread_in_code(&mut self, thread_id: String, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.code.viewer = None;
        d.code.inline.reveal = Some(thread_id);
        self.set_detail_tab(DetailTab::Code, cx);
        if let Some(d) = self.detail.as_mut() {
            d.reindex();
        }
    }

    /// Clear the posting flag, surface errors, then refetch the detail to replace optimistic state.
    fn after_inline(&mut self, rx: futures::channel::oneshot::Receiver<Result<(), String>>, what: &'static str, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let r = rx.await.unwrap_or_else(|_| Err("cancelled".into()));
            this.update(cx, |v, cx| {
                if let Some(d) = v.detail.as_mut() {
                    d.code.inline.posting = false;
                    if let Err(e) = r {
                        d.error = Some(format!("{what}: {e}"));
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
