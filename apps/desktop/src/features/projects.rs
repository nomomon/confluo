//! "Add project" dialog: paste a source, the kind and name are inferred.

use gpui::{div, prelude::*, ClickEvent, Context, PathPromptOptions};

use crate::app::{GitViewer, Overlay};
use crate::state::SourceKind;
use crate::ui::atoms::{Button, ButtonVariant, Chip, ErrorText};
use crate::ui::molecules::{Field, SegmentedTabs};
use crate::ui::theme;

const KINDS: [(&str, SourceKind); 3] = [("Folder", SourceKind::Folder), ("Repo", SourceKind::Repo), ("Group", SourceKind::Group)];

impl GitViewer {
    pub(super) fn render_add_project(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let kinds = KINDS
            .iter()
            .fold(SegmentedTabs::new("add-kind"), |t, (label, k)| t.tab(*label, self.add_kind == *k))
            .on_select(cx.listener(|this, i: &usize, _, cx| {
                this.add_kind = KINDS[*i].1;
                cx.notify();
            }));
        let mut existing = div().flex().flex_wrap().gap(theme::SPACE_1);
        for p in &self.projects {
            let n = p.name.clone();
            existing = existing.child(Chip::new(format!("existing-{}", p.id), p.name.clone()).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.add_name.update(cx, |i, cx| i.set_text(n.clone(), cx));
                },
            )));
        }
        let browse = (self.add_kind == SourceKind::Folder).then(|| {
            Button::new("folder-browse", "Choose...").on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                let rx = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Add".into()) });
                cx.spawn(async move |this, cx| {
                    if let Ok(Ok(Some(paths))) = rx.await {
                        if let Some(p) = paths.into_iter().next() {
                            this.update(cx, |v, cx| v.add_value.update(cx, |i, cx| i.set_text(p.to_string_lossy().to_string(), cx))).ok();
                        }
                    }
                })
                .detach();
            }))
        });
        let value_row = div().flex().gap(theme::SPACE_2).items_center().child(div().flex_1().child(self.add_value.clone())).children(browse);
        div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_3)
            .child(Field::new("Source", value_row))
            .child(Field::new("Kind (detected)", kinds))
            .child(Field::new("Project", div().flex().flex_col().gap(theme::SPACE_1).child(self.add_name.clone()).child(existing)))
            .when_some(self.add_error.clone(), |col, e| col.child(ErrorText::new(e)))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .child(Button::new("add-submit", "Add").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.submit_add_project(cx)))),
            )
    }
}

impl GitViewer {
    pub(super) fn render_rename(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap(theme::SPACE_2)
            .child(div().flex_1().child(self.rename_input.clone()))
            .child(Button::new("rename-save", "Save").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save_rename(cx))))
    }

    pub(super) fn render_confirm_delete(&self, id: i64, cx: &mut Context<Self>) -> impl IntoElement {
        let name = self.projects.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
        div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_3)
            .child(div().text_color(theme::TEXT_MUTED).child(format!("Delete \"{name}\"? Its repos and cached pull requests are removed from git-viewer. Nothing changes on the server.")))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(theme::SPACE_2)
                    .child(Button::new("delete-cancel", "Cancel").variant(ButtonVariant::Ghost).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.overlay = Overlay::None;
                        cx.notify()
                    })))
                    .child(Button::new("delete-confirm", "Delete").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.delete_project(id, cx)))),
            )
    }
}
