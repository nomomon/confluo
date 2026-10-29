//! Root view: owns app state and composes sidebar, list, detail panel and overlays.
//!
//! State transitions live in `state/`, rendering in `features/`.

use gpui::{div, img, prelude::*, Context, Entity, ObjectFit, Window};
use gv_engine::{ChangeRequest, ChangeRequestId, Engine, Involvement, Project, RepoRef};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::settings::{Appearance, Settings};
use crate::state::{DetailState, SourceKind};
use crate::ui::molecules::text_input::{InputEvent, TextInput};
use crate::ui::theme;

/// Which modal dialog is open.
#[derive(Clone, PartialEq)]
pub enum Overlay {
    None,
    Settings,
    AddProject,
    Alias,
}

pub struct GitViewer {
    pub(crate) engine: Arc<Engine>,
    pub(crate) settings: Settings,
    pub(crate) projects: Vec<Project>,
    pub(crate) selected: Option<i64>,
    pub(crate) tab: Involvement,
    pub(crate) needs_you: bool,
    pub(crate) starred_only: bool,
    pub(crate) items: Vec<ChangeRequest>,
    pub(crate) starred: HashSet<ChangeRequestId>,
    pub(crate) aliases: HashMap<RepoRef, String>,
    pub(crate) refreshing: bool,
    pub(crate) status: Option<String>,
    pub(crate) detail: Option<DetailState>,
    pub(crate) overlay: Overlay,
    pub(crate) picker_open: bool,
    pub(crate) copied: bool,
    pub(crate) search: Entity<TextInput>,
    pub(crate) composer: Entity<TextInput>,
    pub(crate) add_name: Entity<TextInput>,
    pub(crate) add_value: Entity<TextInput>,
    pub(crate) add_kind: SourceKind,
    pub(crate) add_error: Option<String>,
    pub(crate) alias_repo: Option<RepoRef>,
    pub(crate) alias_input: Entity<TextInput>,
    pub(crate) bg_input: Entity<TextInput>,
    pub(crate) tint_input: Entity<TextInput>,
}

/// Subscribe `f` to the input's Enter key.
fn on_submit(cx: &mut Context<GitViewer>, input: &Entity<TextInput>, f: impl Fn(&mut GitViewer, Entity<TextInput>, &mut Context<GitViewer>) + 'static) {
    cx.subscribe(input, move |this, input, ev: &InputEvent, cx| {
        if matches!(ev, InputEvent::Submit) {
            f(this, input, cx)
        }
    })
    .detach();
}

impl GitViewer {
    pub fn new(engine: Arc<Engine>, settings: Settings, cx: &mut Context<Self>) -> Self {
        let input = |cx: &mut Context<Self>, ph: &'static str| cx.new(|cx| TextInput::new(ph, cx));
        let search = input(cx, "Search pull requests");
        let composer = input(cx, "Leave a comment, Enter to post");
        let add_name = input(cx, "Project name");
        let add_value = input(cx, "/path/to/folder");
        let alias_input = input(cx, "Alias (empty to clear)");
        let bg_input = input(cx, "/path/to/image.jpg, Enter to apply");
        let tint_input = input(cx, "#14141c");
        bg_input.update(cx, |i, cx| i.set_text(settings.background_image.clone().unwrap_or_default(), cx));
        tint_input.update(cx, |i, cx| i.set_text(settings.tint.clone(), cx));

        cx.subscribe(&search, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Changed) {
                cx.notify()
            }
        })
        .detach();
        on_submit(cx, &composer, |this, _, cx| this.post_comment(cx));
        on_submit(cx, &add_value, |this, _, cx| this.submit_add_project(cx));
        on_submit(cx, &alias_input, |this, _, cx| this.save_alias(cx));
        on_submit(cx, &bg_input, |this, input, cx| {
            let t = input.read(cx).text().trim().to_string();
            this.settings.background_image = (!t.is_empty()).then_some(t);
            this.save_settings(cx);
        });
        on_submit(cx, &tint_input, |this, input, cx| {
            let t = input.read(cx).text();
            if theme::parse_hex(&t).is_some() {
                this.settings.tint = t.trim().to_string();
                this.save_settings(cx);
            }
        });

        let mut this = Self {
            engine,
            settings,
            projects: vec![],
            selected: None,
            tab: Involvement::All,
            needs_you: false,
            starred_only: false,
            items: vec![],
            starred: HashSet::new(),
            aliases: HashMap::new(),
            refreshing: false,
            status: None,
            detail: None,
            overlay: Overlay::None,
            picker_open: false,
            copied: false,
            search,
            composer,
            add_name,
            add_value,
            add_kind: SourceKind::Folder,
            add_error: None,
            alias_repo: None,
            alias_input,
            bg_input,
            tint_input,
        };
        this.reload_projects();
        this.reload();
        this.refresh(cx);
        this
    }

    fn base_alpha(&self) -> f32 {
        match self.settings.appearance {
            Appearance::Opaque => 1.0,
            Appearance::Blurred => 0.35,
            Appearance::Transparent => 0.55,
        }
    }
}

impl Render for GitViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_background_appearance(self.settings.appearance.to_gpui());
        let bg_image = self.settings.background_image.clone().filter(|p| Path::new(p).is_file());
        let main = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .child(self.render_sidebar(cx))
            .child(self.render_list(cx))
            .children(self.detail.as_ref().map(|d| self.render_detail(d, cx)));
        div()
            .relative()
            .size_full()
            .font_family(".SystemUIFont")
            .text_color(theme::text())
            .text_sm()
            .bg(theme::window_bg(&self.settings.tint, self.base_alpha()))
            .when_some(bg_image, |root, p| {
                root.child(img(PathBuf::from(p)).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Cover))
                    .child(div().absolute().top_0().left_0().size_full().bg(gpui::hsla(0., 0., 0., self.settings.background_dim)))
            })
            .child(main)
            .when(self.picker_open, |root| root.child(self.render_picker(cx)))
            .children(self.render_overlay(cx))
    }
}
