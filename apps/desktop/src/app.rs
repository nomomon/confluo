//! Root view: owns app state and composes sidebar, list, detail panel and overlays.
//!
//! State transitions live in `state/`, rendering in `features/`.

use chrono::{DateTime, Utc};
use gpui::{div, img, prelude::*, Context, Entity, ObjectFit, Window};
use gv_engine::{ChangeRequest, ChangeRequestId, Engine, Involvement, Project, RepoRef};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crate::settings::{Appearance, Settings};
use crate::state::{DetailState, HostStatus, SourceKind};
use crate::ui::molecules::text_input::{InputEvent, TextInput};
use crate::ui::theme::{self, Translucency};

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
    /// Set `LOADING_DELAY` after a refresh starts, so quick refreshes show nothing.
    pub(crate) show_loading: bool,
    pub(crate) last_attempt: Option<Instant>,
    pub(crate) refreshed_at: Option<DateTime<Utc>>,
    /// Signed-in hosts; `None` while checking.
    pub(crate) hosts: Option<Vec<HostStatus>>,
    /// One-line error for the banner; cleared on the next successful refresh.
    pub(crate) error: Option<String>,
    pub(crate) detail: Option<DetailState>,
    pub(crate) overlay: Overlay,
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
    pub fn new(engine: Arc<Engine>, settings: Settings, window: &mut Window, cx: &mut Context<Self>) -> Self {
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
            show_loading: false,
            last_attempt: None,
            refreshed_at: None,
            hosts: None,
            error: None,
            detail: None,
            overlay: Overlay::None,
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
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.on_activated(cx)
            }
        })
        .detach();
        this.reload_projects();
        this.refreshed_at = this.engine.last_refreshed(&gv_engine::RefreshTarget::InvolvingMe);
        this.reload();
        this.check_hosts(cx);
        this.refresh(cx);
        this.start_clock(cx);
        this
    }

    /// Panel, card and window fills derived from the appearance settings.
    pub(crate) fn surfaces(&self) -> Translucency {
        let base_alpha = match self.settings.appearance {
            Appearance::Opaque => 1.0,
            Appearance::Blurred => 0.35,
            Appearance::Transparent => 0.55,
        };
        Translucency::new(self.settings.panel_opacity, theme::parse_hex(&self.settings.tint), base_alpha)
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
            .font_family(theme::UI_FONT)
            .text_color(theme::TEXT)
            .text_size(theme::TEXT_SM)
            .bg(self.surfaces().window_bg())
            .when_some(bg_image, |root, p| {
                root.child(img(PathBuf::from(p)).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Cover))
                    .child(div().absolute().top_0().left_0().size_full().bg(theme::SCRIM.opacity(self.settings.background_dim / theme::SCRIM.a)))
            })
            .child(main)
            .children(self.render_overlay(cx))
    }
}
