//! Projects: each project's sources (add, remove), rename and delete, and the repo table with
//! inline aliases and a hide switch.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, Focusable};
use gv_engine::{Project, RepoRef, Source};

use crate::app::GitViewer;
use crate::features::layout;
use crate::features::shared::provider_icon;
use crate::ui::atoms::{Button, ButtonVariant, Icon, IconName, Switch};
use crate::ui::molecules::{Card, EmptyState};
use crate::ui::theme;

fn source_row(s: &Source) -> (AnyElement, String) {
    match s {
        Source::Folder { path, .. } => (Icon::new(IconName::Folder).into_any_element(), path.display().to_string()),
        Source::Repo { url } => (Icon::new(IconName::Link).into_any_element(), url.clone()),
        Source::Group { provider, host, path } => (provider_icon(*provider).into_any_element(), format!("{host}/{path}")),
    }
}

impl GitViewer {
    pub(super) fn render_projects_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let add = Button::new("settings-add-project", "New project").icon(IconName::Plus).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.open_add_project(cx)));
        let mut col = div().flex().flex_col().gap(theme::SPACE_4).pt(theme::SPACE_4).child(div().flex().justify_end().child(add));
        if self.projects.is_empty() {
            col = col.child(EmptyState::new("No projects yet", "Add a folder, repo or group to follow its pull requests"));
        }
        for p in &self.projects {
            col = col.child(self.project_card(p, cx));
        }
        col.child(self.alias_table(cx))
    }

    fn project_card(&self, p: &Project, cx: &mut Context<Self>) -> impl IntoElement {
        let id = p.id;
        let editing = self.renaming == Some(id);
        let title: AnyElement = if editing {
            div().flex().gap(theme::SPACE_2).child(div().flex_1().child(self.rename_input.clone())).child(Button::new(format!("rename-save-{id}"), "Save").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save_rename(cx)))).into_any_element()
        } else {
            div().text_size(theme::text_sm()).font_weight(theme::WEIGHT_MEDIUM).child(p.name.clone()).into_any_element()
        };
        let actions = div()
            .flex()
            .gap(theme::SPACE_1)
            .child(Button::new(format!("src-add-{id}"), "").icon(IconName::Plus).variant(ButtonVariant::Ghost).tooltip("Add source").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.add_source_to(id, cx))))
            .when(!editing, |d| d.child(Button::new(format!("rename-{id}"), "").icon(IconName::Pencil).variant(ButtonVariant::Ghost).tooltip("Rename").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.start_rename(id, cx)))))
            .child(Button::new(format!("delete-{id}"), "").icon(IconName::Trash).variant(ButtonVariant::Ghost).tooltip("Delete project").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.confirm_delete(id, cx))));
        let header = div().flex().items_center().gap(theme::SPACE_3).px(theme::SPACE_4).py(theme::SPACE_3).child(div().flex_1().child(title)).child(actions);
        let sources = p.sources.iter().enumerate().map(|(i, s)| {
            let (icon, text) = source_row(s);
            let source = s.clone();
            div()
                .flex()
                .items_center()
                .gap(theme::SPACE_2P5)
                .px(theme::SPACE_4)
                .py(theme::SPACE_2)
                .child(icon)
                .child(div().flex_1().min_w(theme::SPACE_0).truncate().text_size(theme::text_xs()).font_family(theme::mono_font()).text_color(theme::text_muted()).child(text))
                .child(Button::new(format!("src-rm-{id}-{i}"), "").icon(IconName::X).variant(ButtonVariant::Ghost).tooltip("Remove source").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.remove_source(id, &source, cx))))
                .into_any_element()
        });
        Card::new(self.surfaces()).rows().child(header).children(sources)
    }

    fn alias_table(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let repos = self.all_repos();
        let mut rows: Vec<AnyElement> = vec![div()
            .flex()
            .gap(theme::SPACE_3)
            .px(theme::SPACE_4)
            .py(theme::SPACE_2)
            .text_size(theme::text_xs())
            .text_color(theme::text_subtle())
            .child(div().w(layout::ALIAS_REPO_W).child("Repository"))
            .child(div().flex_1().child("Alias"))
            .child("Hidden")
            .into_any_element()];
        rows.extend(repos.iter().map(|r| self.alias_row(r, cx)));
        if repos.is_empty() {
            rows.push(div().px(theme::SPACE_4).py(theme::SPACE_3).text_size(theme::text_sm()).text_color(theme::text_subtle()).child("No repos yet. Refresh a project first.").into_any_element());
        }
        self.settings_group(Some("Repositories"), rows)
    }

    fn alias_row(&self, r: &RepoRef, cx: &mut Context<Self>) -> AnyElement {
        let o = self.engine.repo_override(r).unwrap_or_default();
        let editing = self.alias_repo.as_ref() == Some(r);
        let (edit_repo, hide_repo) = (r.clone(), r.clone());
        let alias: AnyElement = if editing {
            div().flex_1().child(self.alias_input.clone()).into_any_element()
        } else {
            div()
                .id(gpui::SharedString::from(format!("alias-{r}")))
                .flex_1()
                .px(theme::SPACE_2)
                .py(theme::SPACE_1)
                .rounded(theme::RADIUS_SM)
                .cursor_pointer()
                .hover(|s| s.bg(theme::surface_hover()))
                .text_size(theme::text_sm())
                .text_color(if o.alias.is_some() { theme::text() } else { theme::text_subtle() })
                .child(o.alias.clone().unwrap_or_else(|| "Add alias".into()))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.set_alias_repo(Some(edit_repo.clone()), cx);
                    window.focus(&this.alias_input.focus_handle(cx));
                }))
                .into_any_element()
        };
        div()
            .flex()
            .items_center()
            .gap(theme::SPACE_3)
            .px(theme::SPACE_4)
            .py(theme::SPACE_1P5)
            .child(div().w(layout::ALIAS_REPO_W).flex().items_center().gap(theme::SPACE_2).child(provider_icon(r.kind)).child(div().min_w(theme::SPACE_0).truncate().text_size(theme::text_xs()).font_family(theme::mono_font()).child(r.path.clone())))
            .child(alias)
            .child(Switch::new(format!("hide-{r}"), o.hidden).on_change(cx.listener(move |this, on: &bool, _, cx| this.set_repo_hidden(&hide_repo, *on, cx))))
            .into_any_element()
    }

    /// Alias dialog opened from the detail menu: one input for the PR's repo.
    pub(crate) fn render_alias_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap(theme::SPACE_2)
            .child(div().flex_1().child(self.alias_input.clone()))
            .child(Button::new("alias-save", "Save").variant(ButtonVariant::Primary).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save_alias(cx))))
    }
}
