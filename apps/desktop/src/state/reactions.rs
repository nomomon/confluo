//! Toggling my reaction on the PR or a comment: optimistic, rolled back with a toast on failure.

use gpui::Context;
use gv_engine::{toggle_reaction, ChangeRequestDetail, ReactionGroup, ReactionKind};

use super::{bridge, err_str};
use crate::app::GitViewer;

/// The reaction groups of the PR (`None`) or of one comment.
fn groups<'a>(det: &'a mut ChangeRequestDetail, subject: Option<&str>) -> Option<&'a mut Vec<ReactionGroup>> {
    match subject {
        None => Some(&mut det.reactions),
        Some(id) => det.threads.iter_mut().flat_map(|t| t.comments.iter_mut()).find(|c| c.id == id).map(|c| &mut c.reactions),
    }
}

impl GitViewer {
    pub(crate) fn toggle_reaction(&mut self, subject: Option<String>, kind: ReactionKind, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        let me = d.me.clone().or_else(|| self.engine.me(d.summary.id.repo.kind, &d.summary.id.repo.host)).unwrap_or_default();
        let Some(g) = d.detail.as_mut().and_then(|det| groups(det, subject.as_deref())) else { return };
        let mut save_mru = false;
        let add = toggle_reaction(g, kind, &me);
        let now = g.clone();
        if let Some(cached) = subject.as_ref().and_then(|s| d.ui.awards.get_mut(s)) {
            *cached = now;
        }
        if add {
            // Most recently used reactions lead the comment hover toolbar.
            let mru = &mut self.settings.reaction_mru;
            mru.retain(|k| *k != kind);
            mru.insert(0, kind);
            mru.truncate(8);
            save_mru = true;
        }
        let (engine, id) = (self.engine.clone(), d.summary.id.clone());
        let sub = subject.clone();
        let rx = bridge::spawn(async move { engine.react(&id, sub.as_deref(), kind, add).await.map_err(err_str) });
        cx.spawn(async move |this, cx| {
            if let Ok(Err(e)) = rx.await {
                this.update(cx, |v, cx| {
                    if let Some(g) = v.detail.as_mut().and_then(|d| d.detail.as_mut()).and_then(|det| groups(det, subject.as_deref())) {
                        toggle_reaction(g, kind, &me);
                    }
                    v.show_error_toast("Could not update the reaction", e, cx)
                })
                .ok();
            }
        })
        .detach();
        if save_mru {
            self.save_settings(cx);
        }
        cx.notify();
    }
}
