//! Modal overlay host: picks the dialog for the current `Overlay`.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};

use crate::app::{GitViewer, Overlay};
use crate::ui::molecules::{Dialog, Sheet};

impl GitViewer {
    pub(crate) fn render_overlay(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let name = |id: i64| self.projects.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
        let dialog = match self.overlay {
            Overlay::None => return None,
            Overlay::Settings => return Some(self.render_settings_dialog(cx)),
            Overlay::Palette => return Some(self.render_palette(cx)),
            Overlay::ReviewSummary(action) => self.render_review_dialog(action, cx),
            Overlay::ConfirmClose => self.render_close_dialog(cx),
            Overlay::ConfirmDeleteComment => self.render_delete_comment_dialog(cx),
            Overlay::Merge => self.render_merge_dialog(cx),
            Overlay::ConfirmRerun => self.render_rerun_dialog(cx),
            Overlay::Shortcuts => self.render_shortcuts_sheet(),
            Overlay::AddProject => {
                let close = cx.listener(|this, _: &(), _, cx| {
                    this.overlay = Overlay::None;
                    cx.notify()
                });
                let sheet = Sheet::new("Add project").description("Follow the pull requests of a folder of clones, a repo or a group").on_close(move |w, cx| close(&(), w, cx));
                return Some(sheet.child(self.render_add_project(cx)).into_any_element());
            }
            Overlay::Rename(_) => Dialog::new("Rename project").content(self.render_rename(cx)),
            Overlay::ConfirmDelete(id) => Dialog::new("Delete project")
                .description(format!("Delete \"{}\"? Its repos and cached pull requests are removed from Confluo. Nothing changes on the server.", name(id)))
                .content(div())
                .footer(self.render_confirm_delete_footer(id, cx)),
        };
        Some(
            dialog
                .on_close(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.close_confirm();
                    if this.overlay != Overlay::Settings {
                        this.overlay = Overlay::None;
                    }
                    cx.notify()
                }))
                .into_any_element(),
        )
    }
}
