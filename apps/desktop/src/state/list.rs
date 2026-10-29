//! Project and PR list state: loading from cache, refreshing from providers, stars.

use gpui::Context;
use gv_engine::{ChangeRequestId, Filter, Involvement, ListQuery, RefreshTarget, RepoOverride, RepoRef, StateFilter};
use std::collections::HashSet;

use super::{bridge, err_str};
use crate::app::GitViewer;

impl GitViewer {
    pub(crate) fn reload_projects(&mut self) {
        self.projects = self.engine.projects().unwrap_or_default();
        if let Some(id) = self.selected {
            if !self.projects.iter().any(|p| p.id == id) {
                self.selected = None;
            }
        }
    }

    /// Re-read the list, stars and aliases from the local cache.
    pub(crate) fn reload(&mut self) {
        let q = ListQuery {
            project: self.selected,
            filter: Filter { involvement: self.tab, state: StateFilter::Active },
            starred_only: self.starred_only,
            needs_you_only: self.needs_you,
        };
        match self.engine.list(&q) {
            Ok(v) => self.items = v,
            Err(e) => self.status = Some(err_str(e)),
        }
        let starred_q = ListQuery {
            project: None,
            filter: Filter { involvement: Involvement::All, state: StateFilter::Any },
            starred_only: true,
            needs_you_only: false,
        };
        self.starred = self.engine.list(&starred_q).unwrap_or_default().into_iter().map(|c| c.id).collect();
        self.aliases.clear();
        let repos: HashSet<RepoRef> = self.items.iter().map(|c| c.id.repo.clone()).collect();
        for r in repos {
            if let Ok(RepoOverride { alias: Some(a), .. }) = self.engine.repo_override(&r) {
                self.aliases.insert(r, a);
            }
        }
    }

    /// Fetch from providers in the background, then reload from cache.
    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        let engine = self.engine.clone();
        let target = match self.selected {
            None => RefreshTarget::InvolvingMe,
            Some(id) => RefreshTarget::Project(id),
        };
        let rx = bridge::spawn(async move { engine.refresh(target).await.map_err(err_str) });
        cx.spawn(async move |this, cx| {
            let r = rx.await;
            this.update(cx, |v, cx| {
                v.refreshing = false;
                v.status = match r {
                    Ok(Ok(rep)) if rep.errors.is_empty() => None,
                    Ok(Ok(rep)) => Some(format!(
                        "{} source(s) failed: {}",
                        rep.errors.len(),
                        rep.errors.iter().map(|(l, e)| format!("{l}: {e}")).collect::<Vec<_>>().join("; ")
                    )),
                    Ok(Err(e)) => Some(e),
                    Err(_) => Some("refresh cancelled".into()),
                };
                v.reload();
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn select_project(&mut self, id: Option<i64>, cx: &mut Context<Self>) {
        self.selected = id;
        self.picker_open = false;
        self.detail = None;
        self.reload();
        self.refresh(cx);
    }

    pub(crate) fn toggle_star(&mut self, id: ChangeRequestId, cx: &mut Context<Self>) {
        let on = !self.starred.contains(&id);
        if let Err(e) = self.engine.set_starred(&id, on) {
            self.status = Some(err_str(e));
        }
        self.reload();
        cx.notify();
    }

    pub(crate) fn save_settings(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.settings.save() {
            self.status = Some(format!("saving settings: {e:#}"));
        }
        cx.notify();
    }

    pub(crate) fn repo_label(&self, r: &RepoRef) -> String {
        self.aliases.get(r).cloned().unwrap_or_else(|| r.path.clone())
    }

    pub(crate) fn project_name(&self) -> String {
        match self.selected {
            None => "Inbox".into(),
            Some(id) => self.projects.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default(),
        }
    }
}
