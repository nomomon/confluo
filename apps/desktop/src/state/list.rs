//! Project and PR list state: loading from cache, refreshing from providers, stars.

use gpui::Context;
use gv_engine::{ChangeRequestId, Filter, Involvement, ListQuery, RefreshTarget, RepoOverride, RepoRef, StateFilter};
use std::collections::HashSet;
use std::time::Instant;

use super::{bridge, err_str, timing};
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
            needs_you_only: false,
        };
        match self.engine.list(&q) {
            Ok(v) => self.items = v,
            Err(e) => self.error = Some(err_str(e)),
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

    fn refresh_target(&self) -> RefreshTarget {
        match self.selected {
            None => RefreshTarget::InvolvingMe,
            Some(id) => RefreshTarget::Project(id),
        }
    }

    /// Fetch from providers in the background, then reload from cache.
    /// The cached list stays on screen; a loading hint shows only after `LOADING_DELAY`.
    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        self.last_attempt = Some(Instant::now());
        let engine = self.engine.clone();
        let target = self.refresh_target();
        let rx = bridge::spawn(async move { engine.refresh(target).await.map_err(err_str) });
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(timing::LOADING_DELAY).await;
            this.update(cx, |v, cx| {
                v.show_loading = v.refreshing;
                cx.notify()
            })
            .ok();
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let r = rx.await;
            this.update(cx, |v, cx| {
                v.refreshing = false;
                v.show_loading = false;
                v.error = match r {
                    Ok(Ok(rep)) => v.summarize_errors(&rep.errors),
                    Ok(Err(e)) => v.summarize_errors(&[("refresh".into(), e)]),
                    Err(_) => Some("Refresh cancelled".into()),
                };
                v.refreshed_at = v.engine.last_refreshed(&v.refresh_target());
                v.reload();
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Re-render relative times and refresh when the data is older than `AUTO_REFRESH`.
    pub(crate) fn start_clock(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(timing::TICK).await;
            let alive = this.update(cx, |v, cx| {
                if v.last_attempt.is_none_or(|t| t.elapsed() >= timing::AUTO_REFRESH) {
                    v.refresh(cx);
                }
                cx.notify();
            });
            if alive.is_err() {
                break;
            }
        })
        .detach();
    }

    /// Refresh when the window becomes active, at most once per `ACTIVATE_MIN_GAP`.
    pub(crate) fn on_activated(&mut self, cx: &mut Context<Self>) {
        if self.last_attempt.is_none_or(|t| t.elapsed() >= timing::ACTIVATE_MIN_GAP) {
            self.refresh(cx);
        }
    }

    /// Retry from the error banner: re-check sign-in and refresh.
    pub(crate) fn retry(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.check_hosts(cx);
        self.refresh(cx);
    }

    pub(crate) fn select_project(&mut self, id: Option<i64>, cx: &mut Context<Self>) {
        self.selected = id;
        self.detail = None;
        self.refreshed_at = self.engine.last_refreshed(&self.refresh_target());
        self.reload();
        self.refresh(cx);
    }

    pub(crate) fn toggle_star(&mut self, id: ChangeRequestId, cx: &mut Context<Self>) {
        let on = !self.starred.contains(&id);
        if let Err(e) = self.engine.set_starred(&id, on) {
            self.error = Some(err_str(e));
        }
        self.reload();
        cx.notify();
    }

    pub(crate) fn save_settings(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.settings.save() {
            self.error = Some(format!("Saving settings failed: {e:#}"));
        }
        cx.notify();
    }

    /// The repo alias, else the last path segment (`group/sub/base-front` -> `base-front`).
    pub(crate) fn repo_label(&self, r: &RepoRef) -> String {
        self.aliases.get(r).cloned().unwrap_or_else(|| short_repo(&r.path).to_string())
    }

    pub(crate) fn project_name(&self) -> String {
        match self.selected {
            None => "Inbox".into(),
            Some(id) => self.projects.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default(),
        }
    }
}

fn short_repo(path: &str) -> &str {
    path.trim_end_matches('/').rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::short_repo;

    #[test]
    fn short_repo_keeps_last_segment() {
        assert_eq!(short_repo("acme/projects/demo/base-front"), "base-front");
        assert_eq!(short_repo("solo"), "solo");
    }
}
