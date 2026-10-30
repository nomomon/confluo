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
use crate::state::{BotList, Copied, DetailState, HostStatus, NavState, QuietState, SettingsPage, SourceKind};
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
    /// Window width at the last render, for layout decisions made while rendering children.
    pub(crate) viewport_w: Pixels,
    /// Detail pane open in a narrow window: sidebar collapses to a rail, rows drop secondary meta.
    pub(crate) compact: bool,
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
    /// Stale and snoozed PRs left out of the sections.
    pub(crate) quiet: QuietState,
    /// Sidebar toggle, PR history and the palette highlight.
    pub(crate) nav: NavState,
    /// Opening a PR from history: do not record it again.
    pub(crate) navigating: bool,
    /// Give focus back to the root on the next render (after an overlay closed without a window).
    pub(crate) refocus: bool,
    /// Focus the palette input on the next render (dev hook opens it before a window exists).
    pub(crate) refocus_palette: bool,
    pub(crate) palette_input: Entity<TextInput>,
    /// Summary field of the review dialog.
    pub(crate) review_input: Entity<TextInput>,
    /// Inline title editor, description editor and people picker search of the detail.
    pub(crate) title_input: Entity<TextInput>,
    pub(crate) body_input: Entity<TextInput>,
    pub(crate) people_input: Entity<TextInput>,
    /// Choices in the merge dialog.
    pub(crate) merge_opts: gv_engine::MergeOptions,
    /// Reason under an error toast; None for success toasts.
    pub(crate) toast_error: Option<String>,
    /// Neutral toast (no icon), e.g. "Last PR in Inbox" or why a shortcut did nothing.
    pub(crate) toast_plain: bool,
    /// A keymap action chosen in the palette, dispatched from the root on the next render.
    pub(crate) pending_action: Option<Box<dyn gpui::Action>>,
    /// Project names the app menus were last built with.
    pub(crate) menu_projects: Option<Vec<String>>,
    /// One PR takes the whole window (else triage: list plus preview).
    pub(crate) reviewing: bool,
    /// Cmd+\\ hid the triage preview.
    pub(crate) preview_hidden: bool,
    pub(crate) review: crate::state::ReviewUi,
    /// File tree filter and the Cmd+P file picker query.
    pub(crate) tree_filter: Entity<TextInput>,
    pub(crate) quick_input: Entity<TextInput>,
    pub(crate) refocus_quick: bool,
    /// Right-click menu on a repository name (rename / hide).
    pub(crate) repo_menu: Option<crate::state::RepoMenu>,
    pub(crate) refocus_alias: bool,
    /// Search field of Settings > Repositories.
    pub(crate) repo_search: Entity<TextInput>,
    /// Hold-Cmd shortcut hints (`state::hint_hold`).
    pub(crate) hints: crate::state::hint_hold::HintHold,
    /// Command context snapshot while hints show (read by `hint` helpers during render).
    pub(crate) hint_ctx: Option<crate::state::cmd_ctx::CmdCtx>,
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
        let composer = cx.new(|cx| TextInput::new("Leave a comment", cx).multiline());
        let inline_input = cx.new(|cx| TextInput::new("Leave a comment, \u{2318}Enter to post, Esc to cancel", cx).multiline());
        let add_name = input(cx, "Project name");
        let add_value = input(cx, "Paste a repo or group URL, or a folder path");
        let alias_input = input(cx, "Alias (empty to clear)");
        let rename_input = input(cx, "Project name");
        let bot_authors_input = input(cx, "Add an author, e.g. *-bot, then Enter");
        let bot_branches_input = input(cx, "Add a branch prefix, e.g. deps/, then Enter");
        let palette_input = cx.new(|cx| TextInput::new("Search pull requests, files and commands", cx));
        let review_input = cx.new(|cx| TextInput::new("Leave a summary (optional)", cx).multiline());
        cx.subscribe(&palette_input, |this, _, ev: &InputEvent, cx| match ev {
            InputEvent::Changed => {
                this.nav.palette_sel = 0;
                cx.notify()
            }
            InputEvent::Submit => this.run_palette(this.nav.palette_sel, cx),
        })
        .detach();
        cx.subscribe(&review_input, |this, _, ev: &InputEvent, cx| match ev {
            InputEvent::Changed => cx.notify(),
            InputEvent::Submit => {
                if let Overlay::ReviewSummary(a) = this.overlay {
                    this.submit_review(a, cx)
                }
            }
        })
        .detach();
        let title_input = input(cx, "Title");
        let body_input = cx.new(|cx| TextInput::new("Describe the change (markdown), \u{2318}Enter to save", cx).multiline());
        let people_input = input(cx, "Search people");
        on_submit(cx, &title_input, |this, _, cx| this.save_title(cx));
        on_submit(cx, &body_input, |this, _, cx| this.save_body(cx));
        cx.subscribe(&people_input, |this, _, ev: &InputEvent, cx| match ev {
            InputEvent::Changed => this.search_people(cx),
            InputEvent::Submit => {
                let hl = this.detail.as_ref().map(|d| d.ui.people_hl).unwrap_or(0);
                this.toggle_person(hl, cx)
            }
        })
        .detach();
        on_submit(cx, &bot_authors_input, |this, _, cx| this.add_bot_entry(BotList::Authors, cx));
        on_submit(cx, &bot_branches_input, |this, _, cx| this.add_bot_entry(BotList::Branches, cx));

        cx.subscribe(&search, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Changed) {
                cx.notify()
            }
        })
        .detach();
        on_submit(cx, &composer, |this, _, cx| this.post_comment(cx));
        cx.subscribe(&composer, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Changed) {
                cx.notify()
            }
        })
        .detach();
        on_submit(cx, &inline_input, |this, _, cx| this.submit_inline(cx));
        on_submit(cx, &add_value, |this, _, cx| this.submit_add_project(cx));
        cx.subscribe(&add_value, |this, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Changed) {
                this.on_add_value_changed(cx)
            }
        })
        .detach();
        on_submit(cx, &alias_input, |this, _, cx| {
            if this.repo_menu.is_some() {
                this.save_repo_rename(cx)
            } else {
                this.save_alias(cx)
            }
        });
        on_submit(cx, &rename_input, |this, _, cx| this.save_rename(cx));
        let tree_filter = cx.new(|cx| TextInput::new("Filter files", cx).inset(crate::ui::molecules::INPUT_INSET, crate::ui::molecules::INPUT_INSET));
        let quick_input = cx.new(|cx| TextInput::new("Go to file", cx));
        let repo_search = cx.new(|cx| TextInput::new("Search repositories", cx).inset(crate::ui::molecules::INPUT_INSET, crate::ui::molecules::INPUT_INSET));
        cx.subscribe(&repo_search, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Changed) {
                cx.notify()
            }
        })
        .detach();
        cx.subscribe(&tree_filter, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Changed) {
                cx.notify()
            }
        })
        .detach();
        cx.subscribe(&quick_input, |this, _, ev: &InputEvent, cx| match ev {
            InputEvent::Changed => {
                this.review.quick_sel = 0;
                cx.notify()
            }
            InputEvent::Submit => this.run_quick_file(this.review.quick_sel, cx),
        })
        .detach();
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
            viewport_w: layout::PREVIEW_COMPACT_BELOW,
            compact: false,
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
            quiet: QuietState::default(),
            nav: NavState::default(),
            navigating: false,
            refocus: false,
            refocus_palette: false,
            palette_input,
            review_input,
            title_input,
            body_input,
            people_input,
            merge_opts: Default::default(),
            toast_error: None,
            toast_plain: false,
            pending_action: None,
            menu_projects: None,
            reviewing: false,
            preview_hidden: false,
            review: Default::default(),
            tree_filter,
            quick_input,
            refocus_quick: false,
            repo_menu: None,
            refocus_alias: false,
            repo_search,
            hints: crate::state::hint_hold::HintHold::new(crate::dev::hints()),
            hint_ctx: None,
        };
        cx.observe_window_appearance(window, |_, _, cx| cx.notify()).detach();
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.on_activated(cx)
            } else if this.hints.blur() {
                cx.notify()
            }
        })
        .detach();
        cx.observe_keystrokes(|this, _, _, cx| {
            if this.hints.key_down() {
                cx.notify()
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

    pub(crate) fn sidebar_w(&self) -> Pixels {
        if self.nav.sidebar_hidden {
            gpui::px(0.)
        } else if self.compact {
            layout::SIDEBAR_RAIL_W
        } else {
            layout::SIDEBAR_W
        }
    }

    /// Panel, card and window fills derived from the appearance settings.
    pub(crate) fn surfaces(&self) -> Translucency {
        self.settings.surfaces()
    }
}

impl Render for GitViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.fetch_avatars(cx);
        if std::mem::take(&mut self.refocus) {
            window.focus(&self.focus);
        }
        if std::mem::take(&mut self.refocus_alias) {
            window.focus(&gpui::Focusable::focus_handle(self.alias_input.read(cx), cx));
        }
        if std::mem::take(&mut self.refocus_quick) {
            window.focus(&gpui::Focusable::focus_handle(self.quick_input.read(cx), cx));
        }
        if std::mem::take(&mut self.refocus_palette) {
            window.focus(&gpui::Focusable::focus_handle(self.palette_input.read(cx), cx));
        }
        if let Some(a) = self.pending_action.take() {
            window.focus(&self.focus);
            window.dispatch_action(a, cx);
        }
        let names: Vec<String> = self.projects.iter().map(|p| p.name.clone()).collect();
        if self.menu_projects.as_ref() != Some(&names) {
            crate::features::menus::set_menus(cx, &names);
            self.menu_projects = Some(names);
        }
        theme::set_typography(self.settings.typography());
        theme::set_light(self.settings.theme.is_light(window.appearance()));
        window.set_background_appearance(self.settings.appearance.to_gpui());
        self.viewport_w = window.viewport_size().width;
        let previewing = !self.reviewing && !self.preview_hidden && self.detail.is_some();
        self.compact = previewing && self.viewport_w < layout::PREVIEW_COMPACT_BELOW;
        self.hint_ctx = (self.hints.visible() && self.settings.shortcut_hints).then(|| self.cmd_ctx());
        let bg_image = self.settings.background_image.clone().filter(|p| Path::new(p).is_file());
        let main = div().absolute().top_0().left_0().size_full().flex();
        let main = match self.detail.as_ref().filter(|_| self.reviewing) {
            // Review mode: the PR takes the whole content area; the sidebar and list step aside.
            Some(d) => main.child(self.render_review(d, cx)),
            None => main
                .when(!self.nav.sidebar_hidden, |m| m.child(crate::ui::motion::slide_in_left(div().h_full().child(self.render_sidebar(cx)), "sidebar", layout::SIDEBAR_SLIDE)))
                .child(self.render_list(cx))
                .children(self.detail.as_ref().filter(|_| previewing).map(|d| self.render_preview(d, cx))),
        };
        let root = div().id("root");
        self.key_handlers(root, cx)
            .on_modifiers_changed(cx.listener(Self::hint_modifiers))
            .relative()
            .size_full()
            .font_family(theme::ui_font())
            .text_color(theme::text())
            .text_size(theme::text_sm())
            .line_height(gpui::relative(theme::LINE_HEIGHT))
            .bg(self.surfaces().window_bg())
            .when_some(bg_image, |root, p| {
                root.child(img(PathBuf::from(p)).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Cover))
                    .child(div().absolute().top_0().left_0().size_full().bg(theme::scrim().opacity(self.settings.background_dim / theme::scrim().a)))
            })
            .when(self.review.resizing.is_some(), |root| self.resize_handlers(root, cx))
            .child(main)
            .children(self.toast.clone().map(|t| {
                let toast = match &self.toast_error {
                    Some(why) => Toast::new(t).description(why.clone()).variant(ToastVariant::Error),
                    None if self.toast_plain => Toast::new(t),
                    None => Toast::new(t).variant(ToastVariant::Success),
                };
                let toast = if self.quiet.undo.is_some() { toast.action("Undo", cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.undo_snooze(cx))) } else { toast };
                Toaster::new().toast(toast)
            }))
            .children(self.render_overlay(cx))
            .children(self.render_repo_menu(cx))
    }
}
