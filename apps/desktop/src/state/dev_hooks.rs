//! Dev-only launch hooks (`CONFLUO_VIEW`, `CONFLUO_OVERLAY`, `CONFLUO_SCROLL`); see docs/development.md.

use gpui::Context;

use crate::app::{GitViewer, Menu, Overlay};

impl GitViewer {
    /// Dev only: `CONFLUO_VIEW` picks the list view at launch (see `dev::view`).
    pub(crate) fn apply_dev_view(&mut self) {
        match crate::dev::view().as_deref() {
            Some("starred") => self.starred_only = true,
            Some("bots") => self.show_bots = true,
            Some(v) if v.starts_with("project:") => {
                let name = v["project:".len()..].to_lowercase();
                self.selected = self.projects.iter().find(|p| p.name.to_lowercase() == name).map(|p| p.id);
            }
            _ => {}
        }
    }

    /// Dev only: `CONFLUO_OVERLAY=add-project|rename|delete|menu|palette|review|older` opens that overlay at launch
    /// (rename, delete and menu act on the first project).
    pub(crate) fn overlay_from_env(&mut self, cx: &mut Context<Self>) {
        let first = self.projects.first().map(|p| p.id);
        match (crate::dev::overlay().as_deref(), first) {
            (Some("add-project"), _) => self.open_add_project(cx),
            (Some("rename"), Some(id)) => self.open_rename(id, cx),
            (Some("delete"), Some(id)) => self.confirm_delete(id, cx),
            (Some("menu"), Some(id)) => self.menu = Some(Menu::Project(id)),
            (Some("palette"), _) => {
                self.overlay = Overlay::Palette;
                self.refocus_palette = true;
            }
            (Some("review"), _) => self.menu = Some(Menu::Review),
            (Some("older"), _) => self.quiet.show_stale = true,
            _ => {}
        }
    }

    /// Dev only: `CONFLUO_SCROLL=<px>` scrolls the detail pane once it has content.
    pub(crate) fn scroll_detail_from_env(&mut self, cx: &mut Context<Self>) {
        let Some(y) = crate::dev::detail_scroll() else { return };
        self.detail_scroll.set_offset(gpui::point(gpui::px(0.), gpui::px(-y)));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(1500)).await;
            this.update(cx, |v, cx| {
                v.detail_scroll.set_offset(gpui::point(gpui::px(0.), gpui::px(-y)));
                cx.notify()
            })
            .ok();
        })
        .detach();
    }

}
