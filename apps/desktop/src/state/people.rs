//! Reviewer and assignee picker: debounced user search and optimistic toggles.

use gpui::{Context, Focusable, Window};
use gv_engine::{Engine, PeopleRole, Review, ReviewState, User};

use super::{bridge, timing, DetailState};
use crate::app::GitViewer;

fn same(a: &User, b: &User) -> bool {
    a.username.eq_ignore_ascii_case(&b.username)
}

impl GitViewer {
    pub(crate) fn open_people(&mut self, role: PeopleRole, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        if d.ui.people == Some(role) {
            d.ui.people = None;
            return cx.notify();
        }
        d.ui.people = Some(role);
        d.ui.people_results.clear();
        d.ui.people_hl = 0;
        self.people_input.update(cx, |i, cx| i.set_text("", cx));
        window.focus(&self.people_input.read(cx).focus_handle(cx));
        self.search_people(cx);
    }

    /// Current people of the open detail for `role`.
    pub(crate) fn current_people(d: &DetailState, role: PeopleRole) -> Vec<User> {
        d.detail.as_ref().map(|x| Engine::people(x, role)).unwrap_or_default()
    }

    /// Picker rows: current people first (filtered by the query), then search results.
    pub(crate) fn people_options(&self, d: &DetailState, role: PeopleRole, cx: &gpui::App) -> Vec<(User, bool)> {
        let q = self.people_input.read(cx).text().to_lowercase();
        let current = Self::current_people(d, role);
        let me_match = |u: &User| q.is_empty() || u.username.to_lowercase().contains(&q) || u.name.as_deref().is_some_and(|n| n.to_lowercase().contains(&q));
        let mut out: Vec<(User, bool)> = current.iter().filter(|u| me_match(u)).map(|u| (u.clone(), true)).collect();
        for u in &d.ui.people_results {
            if !out.iter().any(|(x, _)| same(x, u)) && !u.username.starts_with("team:") {
                out.push((u.clone(), false));
            }
        }
        out
    }

    /// Search after `SEARCH_DEBOUNCE`; stale responses are dropped by generation.
    pub(crate) fn search_people(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.ui.people_gen += 1;
        d.ui.people_loading = true;
        d.ui.people_hl = 0;
        let (gen, id, engine) = (d.ui.people_gen, d.summary.id.clone(), self.engine.clone());
        let q = self.people_input.read(cx).text();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(timing::SEARCH_DEBOUNCE).await;
            let current = this.update(cx, |v, _| v.detail.as_ref().is_some_and(|d| d.ui.people_gen == gen)).unwrap_or(false);
            if !current {
                return;
            }
            let rx = bridge::spawn(async move { engine.search_users(&id, &q).await.unwrap_or_default() });
            let users = rx.await.unwrap_or_default();
            this.update(cx, |v, cx| {
                if let Some(d) = v.detail.as_mut().filter(|d| d.ui.people_gen == gen) {
                    d.ui.people_results = users;
                    d.ui.people_loading = false;
                    v.fetch_avatars(cx);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Add or remove the `i`th picker option.
    pub(crate) fn toggle_person(&mut self, i: usize, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_ref() else { return };
        let Some(role) = d.ui.people else { return };
        let Some((user, on)) = self.people_options(d, role, cx).into_iter().nth(i) else { return };
        let mut desired = Self::current_people(d, role);
        if on {
            desired.retain(|u| !same(u, &user));
        } else {
            desired.push(user);
        }
        let (engine, id, want) = (self.engine.clone(), d.summary.id.clone(), desired.clone());
        let apply = move |d: &mut DetailState| match role {
            PeopleRole::Reviewers => {
                d.summary.reviewers.retain(|r| desired.iter().any(|u| same(u, &r.user)));
                for u in &desired {
                    if !d.summary.reviewers.iter().any(|r| same(&r.user, u)) {
                        d.summary.reviewers.push(Review { user: u.clone(), state: ReviewState::Pending });
                    }
                }
                if let Some(x) = d.detail.as_mut() {
                    x.summary.reviewers = d.summary.reviewers.clone();
                }
            }
            PeopleRole::Assignees => {
                if let Some(x) = d.detail.as_mut() {
                    x.edit.assignees = desired;
                }
            }
        };
        let ok = if role == PeopleRole::Reviewers { "Reviewers updated" } else { "Assignees updated" };
        self.run_edit(ok, if role == PeopleRole::Reviewers { "Updating reviewers" } else { "Updating assignees" }, apply, async move { engine.set_people(&id, role, want).await }, cx);
    }
}
