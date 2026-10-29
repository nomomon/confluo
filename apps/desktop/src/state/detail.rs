//! Detail panel state and loading a single PR from the engine.

use chrono::{DateTime, Utc};
use gpui::Context;
use gv_engine::{ChangeRequest, ChangeRequestDetail};

use super::{bridge, err_str, CodeState};
use crate::app::GitViewer;

#[derive(Clone, Copy, PartialEq)]
pub enum DetailTab {
    Summary,
    Code,
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
    pub code: CodeState,
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
            code: CodeState::new(),
        });
        self.detail_scroll.set_offset(gpui::Point::default());
        self.composer.update(cx, |i, cx| i.set_text("", cx));
        self.load_detail(cx);
    }

    /// Switch the detail tab; the Code tab loads its files lazily on first open.
    pub(crate) fn set_detail_tab(&mut self, tab: DetailTab, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.tab = tab;
        if tab == DetailTab::Code && d.code.files.is_none() {
            self.load_code(cx);
        }
        cx.notify();
    }

    /// Dev only: `CONFLUO_OPEN=<pr url>` opens that PR at launch, `CONFLUO_TAB=code|activity` picks the tab.
    pub(crate) fn open_from_env(&mut self, cx: &mut Context<Self>) {
        let Ok(url) = std::env::var("CONFLUO_OPEN") else { return };
        let Ok(id) = self.engine.resolve_id(&url) else { return };
        let cr = self.engine.store().get_change_request(&id).ok().flatten().or_else(|| self.engine.cached_detail(&id).ok().flatten().map(|d| d.summary));
        let Some(cr) = cr else { return };
        self.open_detail(cr, cx);
        let tab = match std::env::var("CONFLUO_TAB").as_deref() {
            Ok("code") => DetailTab::Code,
            Ok("activity") => DetailTab::Activity,
            _ => DetailTab::Summary,
        };
        self.set_detail_tab(tab, cx);
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
                let mut reload_code = false;
                let Some(d) = v.detail.as_mut().filter(|d| d.summary.id == want) else { return };
                d.loading = false;
                if me.is_some() {
                    d.me = me;
                }
                match r {
                    Ok(det) => {
                        // A new head sha means the diff may have moved too.
                        let moved = d.detail.as_ref().map(|x| &x.head_sha) != Some(&det.head_sha);
                        if d.tab == DetailTab::Code && (moved || d.code.files.is_none()) {
                            d.code.files = None;
                            reload_code = true;
                        }
                        d.summary = det.summary.clone();
                        d.detail = Some(det);
                        d.loaded_at = Some(Utc::now());
                        d.error = None;
                    }
                    Err(e) => d.error = Some(e),
                }
                if reload_code {
                    v.load_code(cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
