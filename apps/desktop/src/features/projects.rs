//! "Add to project" dialog: name, source kind and value.

use gpui::{div, prelude::*, ClickEvent, Context, PathPromptOptions};

use crate::app::GitViewer;
use crate::state::SourceKind;
use crate::ui::atoms::{Button, Chip, ErrorText};
use crate::ui::molecules::{Field, SegmentedTabs};
use crate::ui::theme;

const KINDS: [(&str, SourceKind, &str); 3] = [
    ("Folder", SourceKind::Folder, "/path/to/folder"),
    ("Repo URL", SourceKind::Repo, "https://github.com/owner/repo"),
    ("Group", SourceKind::Group, "gitlab:gitlab.com/my-group"),
];

impl GitViewer {
    pub(super) fn render_add_project(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let kinds = KINDS
            .iter()
            .fold(SegmentedTabs::new("add-kind"), |t, (label, k, _)| t.tab(*label, self.add_kind == *k))
            .on_select(cx.listener(|this, i: &usize, _, cx| {
                let (_, kind, ph) = KINDS[*i];
                this.add_kind = kind;
                this.add_value.update(cx, |inp, cx| {
                    inp.set_placeholder(ph);
                    cx.notify()
                });
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
            .child(Field::new("Project", div().flex().flex_col().gap(theme::SPACE_1).child(self.add_name.clone()).child(existing)))
            .child(Field::new("Source", kinds))
            .child(Field::new("Value", value_row))
            .when_some(self.add_error.clone(), |col, e| col.child(ErrorText::new(e)))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .child(Button::new("add-submit", "Add").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.submit_add_project(cx)))),
            )
    }
}
