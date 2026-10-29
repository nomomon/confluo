//! Root view: owns app state and composes sidebar, list, detail panel and overlays.
//!
//! State transitions live in `state/`, rendering in `features/`.

use chrono::{DateTime, Utc};
use gpui::{div, img, prelude::*, Context, Entity, FocusHandle, ObjectFit, Pixels, ScrollHandle, Window};
use std::cell::Cell;
use std::rc::Rc;
use gv_engine::{ChangeRequest, ChangeRequestId, Engine, Involvement, Project, RepoRef};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crate::settings::Settings;
pub use crate::state::{Menu, Overlay};
use crate::state::{Copied, DetailState, HostStatus, Section, SourceKind};
use crate::ui::molecules::text_input::{InputEvent, TextInput};
use crate::features::layout;
use crate::ui::molecules::Toast;
use crate::ui::theme::{self, Translucency};

pub struct GitViewer {
    /// Root focus target for window-level shortcuts.
    pub(crate) focus: FocusHandle,
    pub(crate) engine: Arc<Engine>,
    pub(crate) settings: Settings,
    pub(crate) projects: Vec<Project>,
    pub(crate) selected: Option<i64>,
    pub(crate) tab: Involvement,
    pub(crate) starred_only: bool,
    pub(crate) items: Vec<ChangeRequest>,
    pub(crate) starred: HashSet<ChangeRequestId>,
    pub(crate) aliases: HashMap<RepoRef, String>,
    /// Avatar URL to image; `None` while loading or after a failed fetch.
    pub(crate) avatars: HashMap<String, Option<Arc<gpui::Image>>>,
    pub(crate) refreshing: bool,
    /// Set `LOADING_DELAY` after a refresh starts, so quick refreshes show nothing.
    pub(crate) show_loading: bool,
    pub(crate) last_attempt: Option<Instant>,
    pub(crate) refreshed_at: Option<DateTime<Utc>>,
    /// Signed-in hosts; `None` while checking.
    pub(crate) hosts: Option<Vec<HostStatus>>,
    /// One-line error for the banner; cleared on the next successful refresh.
    pub(crate) error: Option<String>,
    /// Inbox sections the user collapsed.
    pub(crate) collapsed: HashSet<Section>,
    /// Keyboard selection in the list.
    pub(crate) cursor: Option<ChangeRequestId>,
    pub(crate) detail: Option<DetailState>,
    pub(crate) list_scroll: ScrollHandle,
    pub(crate) detail_scroll: ScrollHandle,
    /// Set by j/k; the highlighted row scrolls itself into view on the next paint.
    pub(crate) reveal_cursor: Rc<Cell<bool>>,
    pub(crate) overlay: Overlay,
    pub(crate) menu: Option<Menu>,
    pub(crate) copied: Option<Copied>,
    pub(crate) copied_gen: u64,
    pub(crate) toast: Option<String>,
    pub(crate) toast_gen: u64,
    pub(crate) detail_w: Pixels,
    /// Dragging the detail pane's resize handle.
    pub(crate) resizing: bool,
    pub(crate) search: Entity<TextInput>,
    pub(crate) composer: Entity<TextInput>,
    pub(crate) add_name: Entity<TextInput>,
    pub(crate) add_value: Entity<TextInput>,
    pub(crate) add_kind: SourceKind,
    /// Project name we filled in from the pasted source; replaced while it is unedited.
    pub(crate) auto_name: Option<String>,
    pub(crate) add_error: Option<String>,
    pub(crate) alias_repo: Option<RepoRef>,
    pub(crate) alias_input: Entity<TextInput>,
    pub(crate) rename_input: Entity<TextInput>,
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
        let search = input(cx, "Search title, repo, author or branch  /");
        let composer = cx.new(|cx| TextInput::new("Leave a comment, \u{2318}Enter to post", cx).multiline());
        let add_name = input(cx, "Project name");
        let add_value = input(cx, "Paste a repo or group URL, or a folder path");
        let alias_input = input(cx, "Alias (empty to clear)");
        let rename_input = input(cx, "Project name");

        cx.subscribe(&search, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Changed) {
                cx.notify()
            }
        })
        .detach();
        on_submit(cx, &composer, |this, _, cx| this.post_comment(cx));
        on_submit(cx, &add_value, |this, _, cx| this.submit_add_project(cx));
        cx.subscribe(&add_value, |this, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Changed) {
                this.on_add_value_changed(cx)
            }
        })
        .detach();
        on_submit(cx, &alias_input, |this, _, cx| this.save_alias(cx));
        on_submit(cx, &rename_input, |this, _, cx| this.save_rename(cx));
        let focus = cx.focus_handle();
        window.focus(&focus);
        let mut this = Self {
            focus,
            engine,
            settings,
            projects: vec![],
            selected: None,
            tab: Involvement::All,
            starred_only: false,
            items: vec![],
            starred: HashSet::new(),
            aliases: HashMap::new(),
            avatars: HashMap::new(),
            refreshing: false,
            show_loading: false,
            last_attempt: None,
            refreshed_at: None,
            hosts: None,
            error: None,
            collapsed: Section::ALL.into_iter().filter(|s| s.collapsed_by_default()).collect(),
            cursor: None,
            detail: None,
            list_scroll: ScrollHandle::new(),
            detail_scroll: ScrollHandle::new(),
            reveal_cursor: Rc::new(Cell::new(false)),
            overlay: Overlay::None,
            menu: None,
            copied: None,
            copied_gen: 0,
            toast: None,
            toast_gen: 0,
            detail_w: layout::DETAIL_W,
            resizing: false,
            search,
            composer,
            add_name,
            add_value,
            add_kind: SourceKind::Folder,
            auto_name: None,
            add_error: None,
            alias_repo: None,
            alias_input,
            rename_input,
        };
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.on_activated(cx)
            }
        })
        .detach();
        this.reload_projects();
        this.cursor = this.restored_cursor();
        this.refreshed_at = this.engine.last_refreshed(&gv_engine::RefreshTarget::InvolvingMe);
        this.reload();
        this.check_hosts(cx);
        this.refresh(cx);
        this.start_clock(cx);
        this
    }

    /// Panel, card and window fills derived from the appearance settings.
    pub(crate) fn surfaces(&self) -> Translucency {
        self.settings.surfaces()
    }
}

impl Render for GitViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.fetch_avatars(cx);
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
        let root = div().id("root");
        self.key_handlers(root, cx)
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
            .when(self.resizing, |root| self.resize_handlers(root, cx))
            .child(main)
            .children(self.toast.clone().map(Toast::new))
            .children(self.render_overlay(cx))
    }
}
