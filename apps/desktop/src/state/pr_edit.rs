//! Editing the open PR: title, description, draft, close/reopen and merge, applied
//! optimistically and rolled back with an error toast when the provider refuses.

use gpui::{Context, Focusable, Window};
use gv_engine::{ChangeRequestState, MergeOptions};
use std::collections::{HashMap, HashSet};
use std::future::Future;

use super::{bridge, err_str, DetailState};
use crate::app::{GitViewer, Overlay};

/// A failed check's log tail, loaded on expand.
#[derive(Clone)]
pub enum LogState {
    Loading,
    Lines(Vec<String>),
    Failed(String),
}

/// Per-PR edit and checks UI state.
#[derive(Default)]
pub struct EditUi {
    pub editing_title: bool,
    pub editing_body: bool,
    pub body_preview: bool,
    /// Open people picker.
    pub people: Option<gv_engine::PeopleRole>,
    pub people_results: Vec<gv_engine::User>,
    pub people_loading: bool,
    pub people_gen: u64,
    pub people_hl: usize,
    pub show_passed: bool,
    /// Expanded check rows by check id.
    pub open_checks: HashSet<String>,
    pub logs: HashMap<String, LogState>,
}

impl GitViewer {
    /// Run `job` after applying `apply` to the open detail; restore the snapshot on error.
    pub(crate) fn run_edit<F>(&mut self, ok: &'static str, what: &'static str, apply: impl FnOnce(&mut DetailState), job: F, cx: &mut Context<Self>)
    where
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let Some(d) = self.detail.as_mut() else { return };
        let snapshot = (d.summary.clone(), d.detail.clone());
        let want = d.summary.id.clone();
        apply(d);
        let rx = bridge::spawn(async move { job.await.map_err(err_str) });
        cx.spawn(async move |this, cx| {
            let r = rx.await.unwrap_or_else(|_| Err("cancelled".into()));
            this.update(cx, |v, cx| {
                match r {
                    Ok(()) => v.show_toast(ok, cx),
                    Err(e) => {
                        if let Some(d) = v.detail.as_mut().filter(|d| d.summary.id == want) {
                            (d.summary, d.detail) = snapshot;
                        }
                        v.show_error_toast(format!("{what} failed"), e, cx);
                    }
                }
                v.reload();
                v.load_detail(cx);
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn start_title_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.ui.editing_title = true;
        let title = d.summary.title.clone();
        self.title_input.update(cx, |i, cx| i.set_text(title, cx));
        window.focus(&self.title_input.read(cx).focus_handle(cx));
        cx.notify();
    }

    pub(crate) fn save_title(&mut self, cx: &mut Context<Self>) {
        let text = self.title_input.read(cx).text().trim().to_string();
        let Some(d) = self.detail.as_mut().filter(|d| d.ui.editing_title) else { return };
        d.ui.editing_title = false;
        self.refocus = true;
        if text.is_empty() || text == d.summary.title {
            return cx.notify();
        }
        let (engine, id, t) = (self.engine.clone(), d.summary.id.clone(), text.clone());
        self.run_edit("Title updated", "Saving the title", move |d| d.summary.title = text, async move { engine.update_change_request(&id, Some(&t), None).await }, cx);
    }

    pub(crate) fn start_body_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.ui.editing_body = true;
        d.ui.body_preview = false;
        let body = d.detail.as_ref().map(|x| x.description.clone()).unwrap_or_default();
        self.body_input.update(cx, |i, cx| i.set_text(body, cx));
        window.focus(&self.body_input.read(cx).focus_handle(cx));
        cx.notify();
    }

    pub(crate) fn save_body(&mut self, cx: &mut Context<Self>) {
        let text = self.body_input.read(cx).text();
        let Some(d) = self.detail.as_mut().filter(|d| d.ui.editing_body) else { return };
        d.ui.editing_body = false;
        self.refocus = true;
        if d.detail.as_ref().is_some_and(|x| x.description == text) {
            return cx.notify();
        }
        let (engine, id, b) = (self.engine.clone(), d.summary.id.clone(), text.clone());
        let apply = move |d: &mut DetailState| {
            if let Some(x) = d.detail.as_mut() {
                x.description = text
            }
        };
        self.run_edit("Description updated", "Saving the description", apply, async move { engine.update_change_request(&id, None, Some(&b)).await }, cx);
    }

    /// Esc: leave title/body editing or the people picker. True when something closed.
    pub(crate) fn cancel_edits(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(d) = self.detail.as_mut() else { return false };
        let open = d.ui.editing_title || d.ui.editing_body || d.ui.people.is_some();
        d.ui.editing_title = false;
        d.ui.editing_body = false;
        d.ui.people = None;
        if open {
            cx.notify();
        }
        open
    }

    pub(crate) fn set_draft(&mut self, draft: bool, cx: &mut Context<Self>) {
        self.layer = None;
        let Some(d) = self.detail.as_ref() else { return };
        let (engine, id) = (self.engine.clone(), d.summary.id.clone());
        let ok = if draft { "Marked as draft" } else { "Ready for review" };
        let apply = move |d: &mut DetailState| d.summary.state = if draft { ChangeRequestState::Draft } else { ChangeRequestState::Open };
        self.run_edit(ok, "Changing draft state", apply, async move { engine.set_draft(&id, draft).await }, cx);
    }

    /// Close (after the confirm dialog) or reopen.
    pub(crate) fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.layer = None;
        self.overlay = Overlay::None;
        let Some(d) = self.detail.as_ref() else { return };
        let (engine, id) = (self.engine.clone(), d.summary.id.clone());
        let apply = move |d: &mut DetailState| d.summary.state = if open { ChangeRequestState::Open } else { ChangeRequestState::Closed };
        self.run_edit(if open { "Reopened" } else { "Closed" }, if open { "Reopening" } else { "Closing" }, apply, async move { engine.set_open(&id, open).await }, cx);
    }

    pub(crate) fn open_merge(&mut self, cx: &mut Context<Self>) {
        self.layer = None;
        let Some(caps) = self.detail.as_ref().and_then(|d| d.detail.as_ref()).map(|x| x.edit.caps.clone()) else { return };
        self.merge_opts = MergeOptions {
            method: caps.merge_methods.first().copied().unwrap_or(gv_engine::MergeMethod::Merge),
            squash: caps.squash_default,
            delete_branch: caps.delete_branch_default,
            when_pipeline_succeeds: caps.auto_merge,
        };
        self.overlay = Overlay::Merge;
        cx.notify();
    }

    pub(crate) fn submit_merge(&mut self, cx: &mut Context<Self>) {
        self.overlay = Overlay::None;
        let Some(d) = self.detail.as_ref() else { return };
        let (engine, id, opts) = (self.engine.clone(), d.summary.id.clone(), self.merge_opts);
        let auto = opts.when_pipeline_succeeds;
        let apply = move |d: &mut DetailState| {
            if !auto {
                d.summary.state = ChangeRequestState::Merged
            }
        };
        self.run_edit(if auto { "Will merge when the pipeline succeeds" } else { "Merged" }, "Merging", apply, async move { engine.merge(&id, &opts).await }, cx);
    }
}
