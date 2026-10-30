//! Project and PR list state: loading from cache, refreshing from providers, stars.

use gpui::Context;
use gv_engine::{
    BotFilter, ChangeRequestId, Filter, Involvement, ListQuery, RefreshTarget, RepoOverride,
    RepoRef, StateFilter,
};
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
            filter: Filter {
                involvement: self.tab,
                state: StateFilter::Active,
            },
            starred_only: self.starred_only,
            needs_you_only: false,
            bots: if self.show_bots {
                BotFilter::off()
            } else {
                self.settings.bot_filter()
            },
            stale_days: self.settings.stale_days,
            include_stale: false,
        };
        match self.engine.listing(&q) {
            Ok(l) => {
                let hidden = l.hidden_bots;
                self.items = l.items;
                self.quiet.stale = l.stale;
                self.quiet.snoozed = l.snoozed;
                // While shown, count what the setting would hide, so the footer can offer "Hide".
                self.hidden_bots = if self.show_bots {
                    self.items
                        .iter()
                        .filter(|c| self.settings.bot_filter().hides(c))
                        .count()
                } else {
                    hidden
                };
            }
            Err(e) => self.error = Some(err_str(e)),
        }
        let starred_q = ListQuery {
            project: None,
            filter: Filter {
                involvement: Involvement::All,
                state: StateFilter::Any,
            },
            starred_only: true,
            needs_you_only: false,
            bots: BotFilter::off(),
            stale_days: None,
            include_stale: true,
        };
        self.starred = self
            .engine
            .list(&starred_q)
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.id)
            .collect();
        self.aliases.clear();
        let repos: HashSet<RepoRef> = self
            .items
            .iter()
            .chain(&self.quiet.stale)
            .map(|c| c.id.repo.clone())
            .collect();
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
    /// Coalesced: one refresh at a time, repeats of the same view within `MANUAL_DEBOUNCE` are
    /// ignored, and nothing is sent while a host is rate limited (the cache stays on screen).
    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.refreshing || crate::dev::no_refresh() {
            return;
        }
        let same_view = self.last_target == Some(self.selected);
        if same_view
            && self
                .last_attempt
                .is_some_and(|t| t.elapsed() < timing::MANUAL_DEBOUNCE)
        {
            return;
        }
        self.paused_until = self.engine.paused_until();
        if self.paused_until.is_some() {
            cx.notify();
            return;
        }
        self.refreshing = true;
        self.last_attempt = Some(Instant::now());
        self.last_target = Some(self.selected);
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
                v.paused_until = None;
                v.error = match r {
                    Ok(Ok(rep)) => {
                        v.paused_until = rep.paused_until;
                        v.summarize_errors(&rep.errors)
                    }
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

    /// Re-render relative times and refresh when the data is older than the refresh interval setting.
    pub(crate) fn start_clock(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(timing::TICK).await;
            let alive = this.update(cx, |v, cx| {
                let every =
                    std::time::Duration::from_secs(u64::from(v.settings.refresh_minutes) * 60);
                if v.settings.refresh_minutes > 0
                    && v.last_attempt.is_none_or(|t| t.elapsed() >= every)
                {
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
        if self
            .last_attempt
            .is_none_or(|t| t.elapsed() >= timing::ACTIVATE_MIN_GAP)
        {
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
        if self.settings.last_project != id {
            self.settings.last_project = id;
            self.settings.save().ok();
        }
        self.refreshed_at = self.engine.last_refreshed(&self.refresh_target());
        self.reload();
        let fresh = self.refreshed_at.is_some_and(|t| {
            (chrono::Utc::now() - t)
                .to_std()
                .is_ok_and(|age| age < timing::SELECT_FRESH)
        });
        if !fresh {
            self.refresh(cx);
        }
    }

    /// Show or hide bot PRs for now (the list footer), without changing the setting.
    pub(crate) fn toggle_show_bots(&mut self, cx: &mut Context<Self>) {
        self.show_bots = !self.show_bots;
        self.reload();
        cx.notify();
    }

    /// Save the hidden author and branch prefix lists from their inputs.
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
        self.aliases
            .get(r)
            .cloned()
            .unwrap_or_else(|| short_repo(&r.path).to_string())
    }

    pub(crate) fn project_name(&self) -> String {
        match self.selected {
            None if self.starred_only => "Starred".into(),
            None => "Inbox".into(),
            Some(id) => self
                .projects
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.clone())
                .unwrap_or_default(),
        }
    }
}

fn short_repo(path: &str) -> &str {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
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
