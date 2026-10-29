//! Detail panel state and loading a single PR from the engine.

use chrono::{DateTime, Utc};
use gpui::Context;
use gv_engine::{ChangeRequest, ChangeRequestDetail};

use super::{bridge, err_str};
use crate::app::GitViewer;

#[derive(Clone, Copy, PartialEq)]
pub enum DetailTab {
    Summary,
    Activity,
}

pub struct DetailState {
    pub summary: ChangeRequest,
    pub detail: Option<ChangeRequestDetail>,
    pub loaded_at: Option<DateTime<Utc>>,
    pub loading: bool,
    pub error: Option<String>,
    pub me: Option<String>,
    pub tab: DetailTab,
    pub checks_open: bool,
    pub reply_to: Option<String>,
    pub editing: Option<String>,
    pub posting: bool,
}

impl GitViewer {
    pub(crate) fn open_detail(&mut self, cr: ChangeRequest, cx: &mut Context<Self>) {
        let cached = self.engine.cached_detail(&cr.id).ok().flatten();
        self.detail = Some(DetailState {
            summary: cr,
            detail: cached,
            loaded_at: None,
            loading: false,
            error: None,
            me: None,
            tab: DetailTab::Summary,
            checks_open: false,
            reply_to: None,
            editing: None,
            posting: false,
        });
        self.composer.update(cx, |i, cx| i.set_text("", cx));
        self.load_detail(cx);
    }

    pub(crate) fn close_detail(&mut self, cx: &mut Context<Self>) {
        self.detail = None;
        self.menu = None;
        cx.notify();
    }

    pub(crate) fn load_detail(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.loading = true;
        let id = d.summary.id.clone();
        let engine = self.engine.clone();
        let rx = bridge::spawn(async move {
            let me = engine.whoami(id.repo.kind, &id.repo.host).await.ok().map(|u| u.username);
            (engine.detail(&id, true).await.map_err(err_str), me)
        });
        let want = d.summary.id.clone();
        cx.spawn(async move |this, cx| {
            let Ok((r, me)) = rx.await else { return };
            this.update(cx, |v, cx| {
                let Some(d) = v.detail.as_mut().filter(|d| d.summary.id == want) else { return };
                d.loading = false;
                if me.is_some() {
                    d.me = me;
                }
                match r {
                    Ok(det) => {
                        d.summary = det.summary.clone();
                        d.detail = Some(det);
                        d.loaded_at = Some(Utc::now());
                        d.error = None;
                    }
                    Err(e) => d.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
