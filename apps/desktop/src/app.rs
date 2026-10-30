//! Root view: owns app state and composes sidebar, list, detail panel and overlays.
//!
//! State transitions live in `state/`, rendering in `features/`.

use chrono::{DateTime, Utc};
use gpui::{prelude::*, Context, Entity, FocusHandle, Pixels, ScrollHandle, Window};
use gv_engine::{ChangeRequest, ChangeRequestId, Engine, Involvement, Project, RepoRef};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use crate::features::layout;
use crate::settings::Settings;
use crate::state::{
    BotList, Copied, DetailState, HostStatus, NavState, QuietState, SettingsPage, SourceKind,
};
pub use crate::state::{Layer, Overlay};
use crate::ui::molecules::text_input::{InputEvent, TextInput};

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
    /// Keychain tokens and the "Use token" dialog (Settings > Accounts).
    pub(crate) accounts_ui: crate::features::settings::token_state::AccountsUi,
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
    /// The one transient floating layer (menu, popover, picker); opening one closes the others.
    pub(crate) layer: Option<Layer>,
    /// The layer a press outside just closed, so the same press on its trigger does not reopen it.
    pub(crate) layer_dismissed: Option<(Layer, Instant)>,
    pub(crate) copied: Option<Copied>,
    pub(crate) copied_gen: u64,
    pub(crate) toast: Option<String>,
    pub(crate) toast_gen: u64,
    /// Bumped per row click; a deferred preview hide only runs for its own click.
    pub(crate) row_click_gen: u64,
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
    /// File the toast's "Open log" action opens (crash report from the last run).
    pub(crate) toast_file: Option<std::path::PathBuf>,
    /// A keymap action chosen in the palette, dispatched from the root on the next render.
    pub(crate) pending_action: Option<Box<dyn gpui::Action>>,
    /// Project names and theme the app menus were last built with (Go lists projects, View marks the theme).
    pub(crate) menu_projects: Option<(Vec<String>, crate::settings::ThemeMode, Option<bool>)>,
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
    /// Height of the rendered description, so its editor opens at the same size.
    pub(crate) desc_h: std::rc::Rc<std::cell::Cell<gpui::Pixels>>,
    /// Search field of Settings > Repositories.
    pub(crate) repo_search: Entity<TextInput>,
    /// Hold-Cmd shortcut hints (`state::hint_hold`).
    pub(crate) hints: crate::state::hint_hold::HintHold,
    /// Command context snapshot while hints show (read by `hint` helpers during render).
    pub(crate) hint_ctx: Option<crate::state::cmd_ctx::CmdCtx>,
    /// Self-update: newer release found, download progress.
    pub(crate) update: crate::state::UpdateState,
    /// "Discard changes?" shows over a dialog with typed input that a close request would lose.
    pub(crate) discard_prompt: bool,
    /// Find in diff (Cmd+F in Review mode) and its query field.
    pub(crate) find: crate::state::FindState,
    pub(crate) find_input: Entity<TextInput>,
    /// Ctrl+G line number field of the file viewer.
    pub(crate) goto_input: Entity<TextInput>,
}

/// Subscribe `f` to the input's Enter key.
fn on_submit(
    cx: &mut Context<GitViewer>,
    input: &Entity<TextInput>,
    f: impl Fn(&mut GitViewer, Entity<TextInput>, &mut Context<GitViewer>) + 'static,
) {
    cx.subscribe(input, move |this, input, ev: &InputEvent, cx| {
        if matches!(ev, InputEvent::Submit) {
            f(this, input, cx)
        }
    })
    .detach();
}

impl GitViewer {
    pub fn new(
        engine: Arc<Engine>,
        settings: Settings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (saved_tree_w, saved_right_ratio) =
            (settings.review_tree_w, settings.review_right_ratio);
        let hidden = settings.hidden_panels;
        let input = |cx: &mut Context<Self>, ph: &'static str| cx.new(|cx| TextInput::new(ph, cx));
        let search = cx.new(|cx| {
            TextInput::new("Search title, repo, author or branch  /", cx).inset(
                crate::ui::molecules::INPUT_INSET,
                crate::ui::molecules::INPUT_INSET,
            )
        });
        let composer = cx.new(|cx| TextInput::new("Leave a comment", cx).multiline().markdown());
        let inline_input = cx.new(|cx| {
            TextInput::new("Leave a comment, \u{2318}Enter to post, Esc to cancel", cx)
                .multiline()
                .markdown()
        });
        let add_name = input(cx, "Project name");
        let add_value = input(cx, "Paste a repo or group URL, or a folder path");
        let alias_input = input(cx, "Alias (empty to clear)");
        let rename_input = input(cx, "Project name");
        let bot_authors_input = input(cx, "Add an author, e.g. *-bot, then Enter");
        let bot_branches_input = input(cx, "Add a branch prefix, e.g. deps/, then Enter");
        let palette_input =
            cx.new(|cx| TextInput::new("Search pull requests, files and commands", cx));
        let review_input =
            cx.new(|cx| TextInput::new("Leave a summary (optional)", cx).multiline());
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
        let body_input = cx.new(|cx| {
            TextInput::new("Describe the change (markdown), \u{2318}Enter to save", cx)
                .multiline()
                .markdown()
                .document()
        });
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
        on_submit(cx, &bot_authors_input, |this, _, cx| {
            this.add_bot_entry(BotList::Authors, cx)
        });
        on_submit(cx, &bot_branches_input, |this, _, cx| {
            this.add_bot_entry(BotList::Branches, cx)
        });

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
        let tree_filter = cx.new(|cx| {
            TextInput::new("Filter files", cx).inset(
                crate::ui::molecules::INPUT_INSET,
                crate::ui::molecules::INPUT_INSET,
            )
        });
        let quick_input = cx.new(|cx| TextInput::new("Go to file", cx));
        let repo_search = cx.new(|cx| {
            TextInput::new("Search repositories", cx).inset(
                crate::ui::molecules::INPUT_INSET,
                crate::ui::molecules::INPUT_INSET,
            )
        });
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
        let find_input = cx.new(|cx| {
            TextInput::new("Find in diff", cx).inset(
                crate::ui::molecules::INPUT_INSET,
                crate::ui::molecules::INPUT_INSET,
            )
        });
        cx.subscribe(&find_input, |this, _, ev: &InputEvent, cx| match ev {
            InputEvent::Changed => this.run_find(cx),
            InputEvent::Submit => this.find_step(1, cx),
        })
        .detach();
        let goto_input = input(cx, "Line number");
        on_submit(cx, &goto_input, |this, _, cx| this.submit_goto_line(cx));
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
            accounts_ui: Default::default(),
            error: None,
            cursor: None,
            detail: None,
            code_cache: HashMap::new(),
            list_scroll: ScrollHandle::new(),
            detail_scroll: ScrollHandle::new(),
            reveal_cursor: Rc::new(Cell::new(false)),
            overlay: Overlay::None,
            layer: None,
            layer_dismissed: None,
            copied: None,
            copied_gen: 0,
            toast: None,
            toast_gen: 0,
            row_click_gen: 0,
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
            nav: NavState {
                sidebar_hidden: hidden.sidebar,
                ..Default::default()
            },
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
            toast_file: None,
            pending_action: None,
            menu_projects: None,
            reviewing: false,
            preview_hidden: hidden.preview,
            review: {
                let mut r = crate::state::ReviewUi::with_sizes(
                    gpui::px(saved_tree_w).clamp(layout::TREE_MIN_W, layout::TREE_MAX_W),
                    saved_right_ratio.clamp(layout::RIGHT_RATIO_MIN, layout::RIGHT_RATIO_MAX),
                );
                (r.tree_open, r.right_open) = (!hidden.tree, !hidden.right);
                r
            },
            tree_filter,
            quick_input,
            refocus_quick: false,
            repo_menu: None,
            refocus_alias: false,
            desc_h: Default::default(),
            repo_search,
            hints: crate::state::hint_hold::HintHold::new(crate::dev::hints()),
            hint_ctx: None,
            update: Default::default(),
            discard_prompt: false,
            find: Default::default(),
            find_input,
            goto_input,
        };
        cx.observe_window_appearance(window, |_, _, cx| cx.notify())
            .detach();
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
        if let Some(log) = crate::crash::take_unseen() {
            this.show_crash_toast(log, cx);
        }
        this.reload_projects();
        if this.settings.launch == crate::settings::Launch::LastView {
            this.selected = this
                .settings
                .last_project
                .filter(|id| this.projects.iter().any(|p| p.id == *id));
        }
        this.apply_dev_view();
        this.cursor = this.restored_cursor();
        this.refreshed_at = this
            .engine
            .last_refreshed(&gv_engine::RefreshTarget::InvolvingMe);
        this.reload();
        this.check_hosts(cx);
        this.refresh(cx);
        this.start_clock(cx);
        this.start_update_checks(cx);
        this.open_from_env(cx);
        this.scroll_detail_from_env(cx);
        this.overlay_from_env(cx);
        this.find_from_env(cx);
        if let Some(p) = SettingsPage::from_env() {
            this.open_settings(Some(p), cx);
        }
        this.token_dialog_from_env(cx);
        this
    }
}
