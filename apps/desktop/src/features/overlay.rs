//! Modal overlay host: picks the dialog for the current `Overlay`. A press on any scrim, Esc and the
//! close button go through `request_close_overlay` (dialogs with typed input ask to discard first).

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};

use crate::app::{GitViewer, Overlay};
use crate::ui::atoms::{Button, ButtonVariant};
use crate::ui::molecules::{Dialog, Sheet};

impl GitViewer {
    pub(crate) fn render_overlay(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let base = self.render_overlay_base(cx)?;
        let top = if self.discard_prompt {
            Some(self.render_discard_prompt(cx))
        } else if self.overlay == Overlay::Settings {
            self.render_token_dialog(cx)
        } else {
            None
        };
        let Some(top) = top else {
            return Some(base);
        };
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(base)
                .child(top)
                .into_any_element(),
        )
    }

    fn render_overlay_base(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let name = |id: i64| {
            self.projects
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.clone())
                .unwrap_or_default()
        };
        let close = cx.listener(|this, _: &(), window, cx| this.request_close_overlay(window, cx));
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
                .on_close(move |w, cx| close(&(), w, cx))
                .into_any_element(),
        )
    }

    /// "Discard changes?" over a dialog whose typed input a close would lose.
    fn render_discard_prompt(&self, cx: &mut Context<Self>) -> AnyElement {
        let keep = cx.listener(|this, _: &(), _, cx| this.keep_editing(cx));
        let footer = div()
            .flex()
            .gap(crate::ui::theme::space_2())
            .child(
                Button::new("discard-keep", "Keep editing")
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.keep_editing(cx))),
            )
            .child(
                Button::new("discard-confirm", "Discard")
                    .variant(ButtonVariant::Destructive)
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.close_overlay_now(window, cx)
                    })),
            );
        Dialog::new("Discard changes?")
            .description("What you typed in this dialog will be lost.")
            .footer(footer)
            .on_close(move |w, cx| keep(&(), w, cx))
            .into_any_element()
    }
}
