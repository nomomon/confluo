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
use crate::state::{BotList, Copied, DetailState, HostStatus, SettingsPage, SourceKind};
use crate::ui::molecules::text_input::{InputEvent, TextInput};
use crate::features::layout;
use crate::ui::molecules::{Toast, ToastVariant, Toaster};
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
    /// The view the last refresh was for, so manual refreshes of it can be debounced.
    pub(crate) last_target: Option<Option<i64>>,
    pub(crate) refreshed_at: Option<DateTime<Utc>>,
    /// A host is rate limited; refreshes wait until then and the header says so.
    pub(crate) paused_until: Option<DateTime<Utc>>,
    /// Bot PRs left out of `items` by the filter.
    pub(crate) hidden_bots: usize,
    /// Show bot PRs for now, without changing the setting.
    pub(crate) show_bots: bool,
    pub(crate) bot_authors_input: Entity<TextInput>,
    pub(crate) bot_branches_input: Entity<TextInput>,
    /// Signed-in hosts; `None` while checking.
    pub(crate) hosts: Option<Vec<HostStatus>>,
    /// One-line error for the banner; cleared on the next successful refresh.
    pub(crate) error: Option<String>,
    /// Keyboard selection in the list.
    pub(crate) cursor: Option<ChangeRequestId>,
    pub(crate) detail: Option<DetailState>,
    /// Changed files per PR, so reopening the Code tab is instant.
    /// Code tab shows the file tree beside the diff.
    pub(crate) code_tree: bool,
    pub(crate) code_cache: HashMap<ChangeRequestId, Arc<Vec<gv_engine::ChangedFile>>>,
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
    /// Window width at the last render, for layout decisions made while rendering children.
    pub(crate) viewport_w: Pixels,
    /// Detail pane open in a narrow window: sidebar collapses to a rail, rows drop secondary meta.
    pub(crate) compact: bool,
    /// Dragging the detail pane's resize handle.
    pub(crate) resizing: bool,
    pub(crate) search: Entity<TextInput>,
    pub(crate) composer: Entity<TextInput>,
    /// Shared field of the Code tab's inline composer, reply and edit.
    pub(crate) inline_input: Entity<TextInput>,
    pub(crate) add_name: Entity<TextInput>,
    pub(crate) add_value: Entity<TextInput>,
    pub(crate) add_kind: SourceKind,
    /// Project name we filled in from the pasted source; replaced while it is unedited.
    pub(crate) auto_name: Option<String>,
    pub(crate) add_error: Option<String>,
    pub(crate) alias_repo: Option<RepoRef>,
    pub(crate) alias_input: Entity<TextInput>,
    pub(crate) rename_input: Entity<TextInput>,
    pub(crate) settings_page: SettingsPage,
    /// Project being renamed inline in Settings.
    pub(crate) renaming: Option<i64>,
    /// The delete confirmation returns to Settings.
    pub(crate) confirm_from_settings: bool,
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
        let search = cx.new(|cx| TextInput::new("Search title, repo, author or branch  /", cx).inset(crate::ui::molecules::INPUT_INSET, crate::ui::molecules::INPUT_INSET));
        let composer = cx.new(|cx| TextInput::new("Leave a comment, \u{2318}Enter to post", cx).multiline());
        let inline_input = cx.new(|cx| TextInput::new("Leave a comment, \u{2318}Enter to post, Esc to cancel", cx).multiline());
        let add_name = input(cx, "Project name");
        let add_value = input(cx, "Paste a repo or group URL, or a folder path");
        let alias_input = input(cx, "Alias (empty to clear)");
        let rename_input = input(cx, "Project name");
        let bot_authors_input = input(cx, "Add an author, e.g. *-bot, then Enter");
        let bot_branches_input = input(cx, "Add a branch prefix, e.g. deps/, then Enter");
        on_submit(cx, &bot_authors_input, |this, _, cx| this.add_bot_entry(BotList::Authors, cx));
        on_submit(cx, &bot_branches_input, |this, _, cx| this.add_bot_entry(BotList::Branches, cx));

        cx.subscribe(&search, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Changed) {
                cx.notify()
            }
        })
        .detach();
        on_submit(cx, &composer, |this, _, cx| this.post_comment(cx));
        on_submit(cx, &inline_input, |this, _, cx| this.submit_inline(cx));
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
            last_target: None,
            refreshed_at: None,
            paused_until: None,
            hidden_bots: 0,
            show_bots: false,
            bot_authors_input,
            bot_branches_input,
            hosts: None,
            error: None,
            cursor: None,
            detail: None,
            code_tree: false,
            code_cache: HashMap::new(),
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
            viewport_w: layout::COMPACT_BELOW,
            compact: false,
            resizing: false,
            search,
            composer,
            inline_input,
            add_name,
            add_value,
            add_kind: SourceKind::Folder,
            auto_name: None,
            add_error: None,
            alias_repo: None,
            alias_input,
            rename_input,
            settings_page: SettingsPage::default(),
            renaming: None,
            confirm_from_settings: false,
        };
        cx.observe_window_appearance(window, |_, _, cx| cx.notify()).detach();
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.on_activated(cx)
            }
        })
        .detach();
        this.reload_projects();
        if this.settings.launch == crate::settings::Launch::LastView {
            this.selected = this.settings.last_project.filter(|id| this.projects.iter().any(|p| p.id == *id));
        }
        this.apply_dev_view();
        this.cursor = this.restored_cursor();
        this.refreshed_at = this.engine.last_refreshed(&gv_engine::RefreshTarget::InvolvingMe);
        this.reload();
        this.check_hosts(cx);
        this.refresh(cx);
        this.start_clock(cx);
        this.open_from_env(cx);
        this.scroll_detail_from_env(cx);
        this.overlay_from_env(cx);
        if let Some(p) = SettingsPage::from_env() {
            this.open_settings(Some(p), cx);
        }
        this
    }

    /// Dev only: `CONFLUO_VIEW` picks the list view at launch (see `dev::view`).
    fn apply_dev_view(&mut self) {
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

    /// Dev only: `CONFLUO_OVERLAY=add-project|rename|delete|menu` opens that overlay at launch
    /// (rename, delete and menu act on the first project).
    fn overlay_from_env(&mut self, cx: &mut Context<Self>) {
        let first = self.projects.first().map(|p| p.id);
        match (crate::dev::overlay().as_deref(), first) {
            (Some("add-project"), _) => self.open_add_project(cx),
            (Some("rename"), Some(id)) => self.open_rename(id, cx),
            (Some("delete"), Some(id)) => self.confirm_delete(id, cx),
            (Some("menu"), Some(id)) => self.menu = Some(Menu::Project(id)),
            _ => {}
        }
    }

    /// Dev only: `CONFLUO_SCROLL=<px>` scrolls the detail pane once it has content.
    fn scroll_detail_from_env(&mut self, cx: &mut Context<Self>) {
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

    pub(crate) fn sidebar_w(&self) -> Pixels {
        if self.compact { layout::SIDEBAR_RAIL_W } else { layout::SIDEBAR_W }
    }

    /// The dragged detail width, narrowed so the list keeps `LIST_MIN_W`.
    pub(crate) fn detail_width(&self) -> Pixels {
        self.detail_w.min(self.viewport_w - self.sidebar_w() - layout::LIST_MIN_W).max(layout::DETAIL_MIN_W)
    }

    /// Panel, card and window fills derived from the appearance settings.
    pub(crate) fn surfaces(&self) -> Translucency {
        self.settings.surfaces()
    }
}

impl Render for GitViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.fetch_avatars(cx);
        theme::set_typography(self.settings.typography());
        theme::set_light(self.settings.theme.is_light(window.appearance()));
        window.set_background_appearance(self.settings.appearance.to_gpui());
        self.viewport_w = window.viewport_size().width;
        self.compact = self.detail.is_some() && self.viewport_w < layout::COMPACT_BELOW;
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
            .font_family(theme::ui_font())
            .text_color(theme::text())
            .text_size(theme::text_sm())
            .bg(self.surfaces().window_bg())
            .when_some(bg_image, |root, p| {
                root.child(img(PathBuf::from(p)).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Cover))
                    .child(div().absolute().top_0().left_0().size_full().bg(theme::scrim().opacity(self.settings.background_dim / theme::scrim().a)))
            })
            .when(self.resizing, |root| self.resize_handlers(root, cx))
            .child(main)
            .children(self.toast.clone().map(|t| Toaster::new().toast(Toast::new(t).variant(ToastVariant::Success))))
            .children(self.render_overlay(cx))
    }
}
