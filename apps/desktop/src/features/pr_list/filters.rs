//! Involvement tabs, filter chips and client-side search.

use gpui::{div, prelude::*, App, ClickEvent, Context};
use gv_engine::{ChangeRequest, Involvement};

use crate::app::GitViewer;
use crate::ui::atoms::{Button, ButtonVariant, Chip};
use crate::ui::molecules::SegmentedTabs;
use crate::ui::theme;

const INVOLVEMENTS: [(&str, Involvement); 3] =
    [("Authored", Involvement::Authored), ("Reviewing", Involvement::Reviewing), ("All", Involvement::All)];

impl GitViewer {
    /// Items matching the search box (title, repo, author, alias, branch or number).
    pub(crate) fn visible_items(&self, cx: &App) -> Vec<&ChangeRequest> {
        let q = self.search.read(cx).text().to_lowercase();
        self.items
            .iter()
            .filter(|c| {
                q.is_empty()
                    || c.title.to_lowercase().contains(&q)
                    || c.id.repo.path.to_lowercase().contains(&q)
                    || c.author.username.to_lowercase().contains(&q)
                    || c.source_branch.to_lowercase().contains(&q)
                    || c.target_branch.to_lowercase().contains(&q)
                    || self.aliases.get(&c.id.repo).is_some_and(|a| a.to_lowercase().contains(&q))
                    || c.id.number.to_string() == q.trim_start_matches(['#', '!'])
            })
            .collect()
    }

    /// Search input with a clear button while it has text.
    pub(super) fn render_search(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_text = !self.search.read(cx).text().is_empty();
        div().relative().flex_1().child(self.search.clone()).when(has_text, |d| {
            d.child(
                div().absolute().top_0().bottom_0().right(theme::SPACE_1).flex().items_center().child(
                    Button::new("search-clear", "")
                        .icon("\u{2715}")
                        .variant(ButtonVariant::Ghost)
                        .tooltip("Clear  esc")
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.search.update(cx, |i, cx| i.set_text("", cx));
                            window.focus(&this.focus);
                        })),
                ),
            )
        })
    }

    pub(super) fn render_involvement_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        INVOLVEMENTS
            .iter()
            .fold(SegmentedTabs::new("tabs"), |tabs, (label, inv)| tabs.tab(*label, self.tab == *inv))
            .on_select(cx.listener(|this, i: &usize, _, cx| {
                this.tab = INVOLVEMENTS[*i].1;
                this.reload();
                cx.notify()
            }))
    }

    pub(super) fn render_filter_chips(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap(theme::SPACE_2)
            .child(Chip::new("chip-needs", "Needs you").active(self.needs_you).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.needs_you = !this.needs_you;
                    this.reload();
                    cx.notify()
                },
            )))
            .child(Chip::new("chip-star", "\u{2605} Starred").active(self.starred_only).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.starred_only = !this.starred_only;
                    this.reload();
                    cx.notify()
                },
            )))
    }
}
