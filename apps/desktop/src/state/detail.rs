//! Detail panel state and loading a single PR from the engine.

use chrono::{DateTime, Utc};
use gpui::Context;
use gv_engine::{ChangeRequest, ChangeRequestDetail};

use super::{bridge, err_str, CodeState};
use crate::app::GitViewer;

#[derive(Clone, Copy, PartialEq)]
pub enum DetailTab {
    Summary,
    /// Review mode: the diff fills the center.
    Code,
}

pub struct DetailState {
    pub summary: ChangeRequest,
    pub detail: Option<ChangeRequestDetail>,
    pub loaded_at: Option<DateTime<Utc>>,
    pub loading: bool,
    pub error: Option<String>,
    pub me: Option<String>,
    pub tab: DetailTab,
    /// Editing, people picker and checks expansion.
    pub ui: super::EditUi,
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
            ui: Default::default(),
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

    /// Dev only: `CONFLUO_OPEN=<pr url>` previews that PR at launch, `CONFLUO_MODE=review` reviews it,
    /// `CONFLUO_REVEAL=<thread id>` scrolls the Code tab to that inline thread,
    /// `CONFLUO_SELECT=path:start-end` selects those new-side diff lines.
    pub(crate) fn open_from_env(&mut self, cx: &mut Context<Self>) {
        let Ok(url) = std::env::var("CONFLUO_OPEN") else { return };
        let Ok(id) = self.engine.resolve_id(&url) else { return };
        let cr = self.engine.store().get_change_request(&id).ok().flatten().or_else(|| self.engine.cached_detail(&id).ok().flatten().map(|d| d.summary));
        let Some(cr) = cr else { return };
        self.open_detail(cr, cx);
        // `CONFLUO_MODE=review` (or `CONFLUO_TAB=code|activity`) enters Review mode; activity opens Conversation.
        let tab = std::env::var("CONFLUO_TAB").ok();
        if std::env::var("CONFLUO_MODE").as_deref() == Ok("review") || tab.is_some() {
            self.enter_review(cx);
            if tab.as_deref() == Some("activity") {
                self.review.right_tab = super::RightTab::Conversation;
            }
        }
        if let (Ok(t), Some(d)) = (std::env::var("CONFLUO_REVEAL"), self.detail.as_mut()) {
            d.code.inline.reveal = Some(t);
        }
        if let Some(d) = self.detail.as_mut() {
            d.code.pending_select = std::env::var("CONFLUO_SELECT").ok().and_then(|s| super::line_select::PendingSelect::parse(&s));
            d.code.apply_pending_select();
        }
    }

    pub(crate) fn close_detail(&mut self, cx: &mut Context<Self>) {
        self.reviewing = false;
        self.nav.last_closed = self.detail.take().map(|d| d.summary);
        self.layer = None;
        cx.notify();
    }

    pub(crate) fn load_detail(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.loading = true;
        let id = d.summary.id.clone();
        let engine = self.engine.clone();
        let cached_me = self.engine.me(id.repo.kind, &id.repo.host);
        let rx = bridge::spawn(async move {
            // The cached username is enough; asking the host first would queue behind a running refresh.
            let me = match cached_me {
                Some(u) => Some(u),
                None => engine.whoami(id.repo.kind, &id.repo.host).await.ok().map(|u| u.username),
            };
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
                        d.apply_awards();
                        d.loaded_at = Some(Utc::now());
                        d.error = None;
                        d.reindex();
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
