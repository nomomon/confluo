//! Dev-only launch hooks (`CONFLUO_VIEW`, `CONFLUO_OVERLAY`, `CONFLUO_SCROLL`); see docs/development.md.

use gpui::Context;

use crate::app::{GitViewer, Layer, Overlay};

impl GitViewer {
    /// Dev only: `CONFLUO_VIEW` picks the list view at launch (see `dev::view`).
    pub(crate) fn apply_dev_view(&mut self) {
        match crate::dev::view().as_deref() {
            Some("starred") => self.starred_only = true,
            Some("bots") => self.show_bots = true,
            Some(v) if v.starts_with("project:") => {
                let name = v["project:".len()..].to_lowercase();
                self.selected = self
                    .projects
                    .iter()
                    .find(|p| p.name.to_lowercase() == name)
                    .map(|p| p.id);
            }
            _ => {}
        }
    }

    /// Dev only: `CONFLUO_OVERLAY=add-project|rename|delete|menu|palette|shortcuts|review|older|more|merge|close|people|checks`
    /// (`CONFLUO_PALETTE_QUERY` prefills the palette),
    /// also `checks-popover|quick-open|repo-menu|repo-rename` (Review mode popovers and the repository menu)
    /// opens that overlay at launch (people opens the reviewer picker, checks expands the first failed check)
    /// (rename, delete and menu act on the first project).
    pub(crate) fn overlay_from_env(&mut self, cx: &mut Context<Self>) {
        let first = self.projects.first().map(|p| p.id);
        // `CONFLUO_THREADS=unresolved|needs` sets the Conversation filter.
        match std::env::var("CONFLUO_THREADS").as_deref() {
            Ok("unresolved") => self.review.thread_filter = super::ThreadFilter::Unresolved,
            Ok("needs") => self.review.thread_filter = super::ThreadFilter::NeedsMe,
            _ => {}
        }
        match (crate::dev::overlay().as_deref(), first) {
            (Some("add-project"), _) => self.open_add_project(cx),
            (Some("rename"), Some(id)) => self.open_rename(id, cx),
            (Some("delete"), Some(id)) => self.confirm_delete(id, cx),
            (Some("menu"), Some(id)) => self.layer = Some(Layer::Project(id)),
            (Some("palette"), _) => {
                self.overlay = Overlay::Palette;
                self.refocus_palette = true;
                if let Ok(q) = std::env::var("CONFLUO_PALETTE_QUERY") {
                    self.palette_input.update(cx, |i, cx| i.set_text(q, cx));
                }
            }
            (Some("shortcuts"), _) => self.overlay = Overlay::Shortcuts,
            (Some("review"), _) => self.layer = Some(Layer::Review),
            (Some("more"), _) => self.layer = Some(Layer::DetailMore),
            (Some("merge"), _) => self.open_merge(cx),
            (Some("close"), _) => self.overlay = Overlay::ConfirmClose,
            (Some("people"), _) => {
                if let Some(d) = self.detail.as_mut() {
                    d.ui.people = Some(gv_engine::PeopleRole::Reviewers);
                }
                self.search_people(cx);
            }
            (Some("checks"), _) => {
                let failed = self
                    .detail
                    .as_ref()
                    .and_then(|d| d.detail.as_ref())
                    .and_then(|x| {
                        x.checks
                            .runs
                            .iter()
                            .find(|r| r.has_log)
                            .map(|r| r.id.clone())
                    });
                if let Some(id) = failed {
                    self.toggle_check(id, cx);
                }
            }
            (Some("checks-popover"), _) => self.layer = Some(Layer::Checks),
            (Some("quick-open"), _) => self.open_quick_file(cx),
            (Some(o @ ("repo-menu" | "repo-rename")), _) => {
                if let Some(repo) = self.detail.as_ref().map(|d| d.summary.id.repo.clone()) {
                    let pos = gpui::point(
                        gpui::px(if self.reviewing { 720. } else { 420. }),
                        gpui::px(if self.reviewing { 40. } else { 160. }),
                    );
                    self.open_repo_menu(repo, pos, cx);
                    if o == "repo-rename" {
                        self.start_repo_rename(cx);
                    }
                }
            }
            (Some("older"), _) => {
                self.quiet.show_stale = true;
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(1500))
                        .await;
                    this.update(cx, |v, cx| {
                        let max = v.list_scroll.max_offset().height;
                        v.list_scroll.set_offset(gpui::point(gpui::px(0.), -max));
                        cx.notify()
                    })
                    .ok();
                })
                .detach();
            }
            _ => {}
        }
    }

    /// Dev only: `CONFLUO_SCROLL=<px>` scrolls the detail pane once it has content.
    pub(crate) fn scroll_detail_from_env(&mut self, cx: &mut Context<Self>) {
        let Some(y) = crate::dev::detail_scroll() else {
            return;
        };
        self.detail_scroll
            .set_offset(gpui::point(gpui::px(0.), gpui::px(-y)));
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1500))
                .await;
            this.update(cx, |v, cx| {
                v.detail_scroll
                    .set_offset(gpui::point(gpui::px(0.), gpui::px(-y)));
                cx.notify()
            })
            .ok();
        })
        .detach();
    }
}
