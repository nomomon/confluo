//! Repositories: rename how a repository appears (an alias, local only) or hide it. A searchable
//! list grouped by host; click a name to edit it inline, Enter saves.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};
use gv_engine::RepoRef;

use crate::app::GitViewer;
use crate::features::shared::provider_icon;
use crate::ui::atoms::{tooltip, IconName, Switch};
use crate::ui::molecules::command_palette::score;
use crate::ui::molecules::Input;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_repositories(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let q = self.repo_search.read(cx).text();
        let counts = self.engine.repo_pr_counts();
        let mut repos: Vec<RepoRef> = self.all_repos();
        repos.extend(counts.keys().cloned());
        repos.sort_by(|a, b| (&a.host, &a.path).cmp(&(&b.host, &b.path)));
        repos.dedup();
        let repos: Vec<RepoRef> = repos.into_iter().filter(|r| score(&q, &format!("{} {}", self.repo_label(r), r.path)).is_some()).collect();
        let search = Input::new("repo-search", self.repo_search.clone(), !q.is_empty()).leading(IconName::Search).on_clear(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.repo_search.update(cx, |i, cx| i.set_text("", cx));
        }));
        let mut col = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(theme::SPACE_2)
            .pt(theme::SPACE_1)
            .child(div().text_size(theme::text_sm()).text_color(theme::text_muted()).child("Rename how a repository appears in Confluo. This doesn't change anything on GitHub or GitLab."))
            .child(div().w_full().flex().pt(theme::SPACE_2).child(div().flex_1().child(search)));
        if repos.is_empty() {
            col = col.child(div().pt(theme::SPACE_3).text_size(theme::text_sm()).text_color(theme::text_subtle()).child(if q.is_empty() { "No repositories yet. Refresh a project first." } else { "No matching repositories" }));
        }
        let mut hosts: Vec<&str> = repos.iter().map(|r| r.host.as_str()).collect();
        hosts.dedup();
        for host in hosts {
            let rows: Vec<AnyElement> = repos.iter().filter(|r| r.host == host).map(|r| self.repo_row(r, counts.get(r).copied().unwrap_or(0), cx)).collect();
            col = col
                .child(div().pt(theme::SPACE_3).px(theme::SPACE_1).text_size(theme::text_xs()).font_weight(theme::WEIGHT_MEDIUM).text_color(theme::text_subtle()).child(host.to_string()))
                .child(div().w_full().flex().flex_col().mt(-theme::SPACE_4).child(self.settings_group(None, rows)));
        }
        col
    }

    fn repo_row(&self, r: &RepoRef, prs: usize, cx: &mut Context<Self>) -> AnyElement {
        let o = self.engine.repo_override(r).unwrap_or_default();
        let editing = self.alias_repo.as_ref() == Some(r);
        let (edit_repo, hide_repo) = (r.clone(), r.clone());
        let name: AnyElement = if editing {
            div().w_full().rounded(theme::RADIUS_MD).border_1().border_color(theme::focus_ring()).bg(theme::surface()).py(theme::SPACE_0P5).child(self.alias_input.clone()).into_any_element()
        } else {
            div()
                .id(gpui::SharedString::from(format!("repo-name-{r}")))
                .px(theme::SPACE_1)
                .rounded(theme::RADIUS_SM)
                .cursor_text()
                .hover(|s| s.bg(theme::surface_hover()))
                .tooltip(tooltip("Click to rename"))
                .text_size(theme::text_sm())
                .font_weight(theme::WEIGHT_MEDIUM)
                .text_color(if o.hidden { theme::text_subtle() } else { theme::text() })
                .child(self.repo_label(r))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.set_alias_repo(Some(edit_repo.clone()), cx);
                    window.focus(&gpui::Focusable::focus_handle(this.alias_input.read(cx), cx));
                }))
                .into_any_element()
        };
        div()
            .w_full()
            .flex()
            .items_center()
            .gap(theme::SPACE_3)
            .px(theme::SPACE_4)
            .py(theme::SPACE_2)
            .child(provider_icon(r.kind))
            .child(div().flex().flex_col().flex_1().min_w(theme::SPACE_0).gap(theme::SPACE_0P5).child(name).child(div().px(theme::SPACE_1).truncate().text_size(theme::text_xs()).text_color(theme::text_subtle()).child(r.path.clone())))
            .child(div().flex_shrink_0().text_size(theme::text_xs()).text_color(theme::text_muted()).child(match prs {
                0 => "No PRs".to_string(),
                1 => "1 PR".to_string(),
                n => format!("{n} PRs"),
            }))
            .child(
                div()
                    .id(gpui::SharedString::from(format!("hide-wrap-{r}")))
                    .tooltip(tooltip(if o.hidden { "Hidden from lists" } else { "Hide from lists" }))
                    .child(Switch::new(format!("hide-{r}"), o.hidden).on_change(cx.listener(move |this, on: &bool, _, cx| this.set_repo_hidden(&hide_repo, *on, cx)))),
            )
            .into_any_element()
    }
}
