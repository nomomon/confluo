//! Involvement tabs and client-side search.

use gpui::{prelude::*, App, ClickEvent, Context};
use gv_engine::ChangeRequest;

use crate::app::GitViewer;
use crate::ui::atoms::IconName;
use crate::ui::molecules::{Input, Tabs};

use crate::state::INVOLVEMENTS;

impl GitViewer {
    /// Items matching the search box (title, repo, author, alias, branch or number).
    pub(crate) fn visible_items(&self, cx: &App) -> Vec<&ChangeRequest> {
        let q = self.search.read(cx).text().to_lowercase();
        self.items
            .iter()
            .filter(|c| self.matches_query(c, &q))
            .collect()
    }

    /// Search match on title, repo, alias, author, branches or number (`q` lowercased).
    pub(crate) fn matches_query(&self, c: &ChangeRequest, q: &str) -> bool {
        q.is_empty()
            || c.title.to_lowercase().contains(q)
            || c.id.repo.path.to_lowercase().contains(q)
            || c.author.username.to_lowercase().contains(q)
            || c.source_branch.to_lowercase().contains(q)
            || c.target_branch.to_lowercase().contains(q)
            || self
                .aliases
                .get(&c.id.repo)
                .is_some_and(|a| a.to_lowercase().contains(q))
            || c.id.number.to_string() == q.trim_start_matches(['#', '!'])
    }

    /// Search input with a clear button while it has text.
    pub(super) fn render_search(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_text = !self.search.read(cx).text().is_empty();
        let input = Input::new("search", self.search.clone(), has_text)
            .leading(IconName::Search)
            .clear_tooltip("Clear  esc")
            .on_clear(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.search.update(cx, |i, cx| i.set_text("", cx));
                window.focus(&this.focus);
            }));
        self.hinted::<crate::features::keymap::FocusSearch>(
            input,
            crate::ui::atoms::HintAnchor::Inline,
        )
        .flex()
        .flex_1()
    }

    pub(super) fn render_involvement_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        INVOLVEMENTS
            .iter()
            .fold(Tabs::new("tabs"), |tabs, (label, inv)| {
                tabs.tab(*label, self.tab == *inv)
            })
            .progress(self.review.swipe.tabs.progress)
            .on_select(cx.listener(|this, i: &usize, _, cx| this.switch_list_tab(*i, true, cx)))
    }
}
