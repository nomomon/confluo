//! Command palette, the hub: every keymap entry is a command (shown when available, with its
//! shortcut), plus views, Settings pages, PRs and, in Review mode, the files of the open PR.
//! Ranking (prefixes, fuzzy, MRU) lives in `state::palette_rank`.

use gpui::{AnyElement, App, Context, Focusable, IntoElement, SharedString};
use gv_engine::ChangeRequest;

use crate::app::{GitViewer, Overlay};
use crate::features::keymap::{self, Pal, ENTRIES};
use crate::features::shared::{number_label, state_icon_name};
use crate::state::palette_rank::{push_mru, rank, split_prefix, Cand, Group, Prefix};
use crate::state::SettingsPage;
use crate::ui::atoms::IconName;
use crate::ui::molecules::{CommandPalette, PaletteItem};

/// Footer hint while the query is empty.
const PREFIX_HINT: &str =
    "Type > for actions, # for PRs, @ for files, ~ for recent, ? for shortcuts";
/// Recent PRs collected (an empty query shows `RECENT_SHOWN` of them, `~` all).
const RECENT_MAX: usize = 30;

/// What a palette row does.
#[derive(Clone)]
pub enum PaletteCmd {
    /// A keymap action by name, dispatched like its shortcut.
    Action(&'static str),
    Project(i64),
    Settings(SettingsPage),
    Open(Box<ChangeRequest>),
    /// File index of the open PR: scroll the diff there.
    File(usize),
}

struct Row {
    cmd: PaletteCmd,
    item: PaletteItem,
    cand: Cand,
}

#[allow(clippy::too_many_arguments)] // one flat call per row kind reads better than a builder here
fn row(
    cmd: PaletteCmd,
    key: String,
    group: Group,
    icon: IconName,
    title: String,
    detail: Option<String>,
    shortcut: String,
    text: String,
    number: Option<u64>,
) -> Row {
    let has_shortcut = !shortcut.is_empty();
    Row {
        cmd,
        item: PaletteItem {
            icon,
            title: title.into(),
            detail: detail.map(SharedString::from),
            shortcut: has_shortcut.then(|| shortcut.into()),
            group: group.title(),
        },
        cand: Cand {
            key,
            group,
            text,
            number,
            has_shortcut,
        },
    }
}

/// The palette action that picks `mode` (marked "Current").
fn theme_action(mode: crate::settings::ThemeMode) -> &'static str {
    match mode {
        crate::settings::ThemeMode::Light => "ThemeLight",
        crate::settings::ThemeMode::Dark => "ThemeDark",
        crate::settings::ThemeMode::System => "ThemeSystem",
    }
}

impl GitViewer {
    fn pr_row(&self, c: &ChangeRequest, group: Group) -> Row {
        let repo = self.repo_label(&c.id.repo);
        let text = format!(
            "{} {} {} #{} {}",
            c.title, repo, c.id.repo.path, c.id.number, c.author.username
        );
        let detail = format!("{repo} {}", number_label(c));
        row(
            PaletteCmd::Open(Box::new(c.clone())),
            format!("pr:{}", c.web_url),
            group,
            state_icon_name(c.state),
            c.title.clone(),
            Some(detail),
            String::new(),
            text,
            Some(c.id.number),
        )
    }

    /// Every candidate row for the current context (ranking picks and orders them).
    fn palette_rows(&self) -> Vec<Row> {
        let ctx = self.cmd_ctx();
        let mut rows = vec![];
        for e in ENTRIES
            .iter()
            .filter(|e| e.pal != Pal::No && (e.available)(&ctx))
        {
            let group = match e.pal {
                Pal::Suggest => Group::Suggested,
                Pal::Go => Group::GoTo,
                _ => Group::Actions,
            };
            let title = if e.pal == Pal::Go {
                format!("Go to {}", e.label)
            } else {
                e.label.to_string()
            };
            let current = theme_action(self.settings.theme) == e.action;
            rows.push(row(
                PaletteCmd::Action(e.action),
                format!("a:{}", e.action),
                group,
                if current { IconName::Check } else { e.icon },
                title.clone(),
                current.then(|| "Current".to_string()),
                e.shortcut(),
                title,
                None,
            ));
        }
        for (i, p) in self.projects.iter().enumerate() {
            let sc = keymap::project_entry(i)
                .map(|e| e.shortcut())
                .unwrap_or_default();
            let title = format!("Go to {}", p.name);
            rows.push(row(
                PaletteCmd::Project(p.id),
                format!("go:project:{}", p.id),
                Group::GoTo,
                IconName::Folder,
                title.clone(),
                None,
                sc,
                title,
                None,
            ));
        }
        for p in SettingsPage::ALL {
            let title = format!("Settings: {}", p.label());
            rows.push(row(
                PaletteCmd::Settings(p),
                format!("go:settings:{}", p.label()),
                Group::GoTo,
                IconName::Settings,
                title.clone(),
                None,
                String::new(),
                title,
                None,
            ));
        }
        let current = self.detail.as_ref().map(|d| d.summary.id.clone());
        let mut seen = vec![];
        for id in self.nav.history.iter().rev().flatten() {
            if Some(id) == current.as_ref() || seen.contains(id) || seen.len() >= RECENT_MAX {
                continue;
            }
            seen.push(id.clone());
            if let Some(c) = self
                .find_item(id)
                .or_else(|| self.engine.store().get_change_request(id).ok().flatten())
            {
                rows.push(self.pr_row(&c, Group::Recent));
            }
        }
        for c in self.items.iter().chain(&self.quiet.stale) {
            rows.push(self.pr_row(c, Group::Prs));
        }
        if let Some(files) = self
            .detail
            .as_ref()
            .filter(|_| ctx.reviewing)
            .and_then(|d| d.code.files.clone())
        {
            for (i, f) in files.iter().enumerate() {
                let p = f.path();
                let (dir, name) = p.rsplit_once('/').unwrap_or(("", p));
                rows.push(row(
                    PaletteCmd::File(i),
                    format!("f:{p}"),
                    Group::Files,
                    IconName::FileCode,
                    name.to_string(),
                    Some(dir.to_string()),
                    String::new(),
                    p.to_string(),
                    None,
                ));
            }
        }
        rows
    }

    /// Rows for the current query, ranked.
    pub(crate) fn palette_entries(&self, cx: &App) -> Vec<(PaletteCmd, PaletteItem)> {
        let q = self.palette_input.read(cx).text();
        let mut rows: Vec<Option<Row>> = self.palette_rows().into_iter().map(Some).collect();
        let cands: Vec<Cand> = rows.iter().flatten().map(|r| r.cand.clone()).collect();
        rank(&cands, &q, &self.settings.palette_mru)
            .into_iter()
            .filter_map(|i| rows[i].take())
            .map(|r| (r.cmd, r.item))
            .collect()
    }

    fn palette_key(&self, i: usize, cx: &App) -> Option<(PaletteCmd, String)> {
        let q = self.palette_input.read(cx).text();
        let rows = self.palette_rows();
        let cands: Vec<Cand> = rows.iter().map(|r| r.cand.clone()).collect();
        let idx = *rank(&cands, &q, &self.settings.palette_mru).get(i)?;
        let r = rows.into_iter().nth(idx)?;
        Some((r.cmd, r.cand.key))
    }

    pub(crate) fn open_palette(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        if self.overlay == Overlay::Palette {
            self.overlay = Overlay::None;
            window.focus(&self.focus);
            cx.notify();
        } else {
            self.open_palette_with("", window, cx)
        }
    }

    /// Open the palette with `query` typed (Ctrl+R: `~ ` lists recent pull requests).
    pub(crate) fn open_palette_with(
        &mut self,
        query: &str,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        self.layer = None;
        self.discard_prompt = false;
        self.overlay = Overlay::Palette;
        self.nav.palette_sel = 0;
        let q = query.to_string();
        self.palette_input.update(cx, |i, cx| i.set_text(q, cx));
        window.focus(&self.palette_input.read(cx).focus_handle(cx));
        cx.notify();
    }

    pub(crate) fn palette_move(&mut self, delta: isize, cx: &mut Context<Self>) {
        let n = self.palette_entries(cx).len().min(50);
        if n > 0 {
            self.nav.palette_sel =
                (self.nav.palette_sel as isize + delta).rem_euclid(n as isize) as usize;
        }
        cx.notify();
    }

    /// Run row `i` (Cmd+Enter: `alt`, opens a PR in the browser); it moves to the front of the MRU list.
    pub(crate) fn run_palette_with(&mut self, i: usize, alt: bool, cx: &mut Context<Self>) {
        let Some((cmd, key)) = self.palette_key(i, cx) else {
            return;
        };
        self.overlay = Overlay::None;
        self.refocus = true;
        push_mru(&mut self.settings.palette_mru, &key);
        self.save_settings(cx);
        match cmd {
            PaletteCmd::Open(c) if alt => cx.open_url(&c.web_url),
            PaletteCmd::Action(name) => {
                self.pending_action = keymap::entry(name).map(|e| (e.build)())
            }
            PaletteCmd::Project(id) => {
                if self.reviewing {
                    self.exit_review(cx);
                }
                self.starred_only = false;
                self.select_project(Some(id), cx)
            }
            PaletteCmd::Settings(p) => self.open_settings(Some(p), cx),
            PaletteCmd::Open(c) => {
                if self.reviewing {
                    self.remember_position();
                    self.review.forget(&c.id);
                    self.set_cursor(c.id.clone(), cx);
                    self.open_detail(*c, cx);
                    self.enter_review(cx);
                } else {
                    self.set_cursor(c.id.clone(), cx);
                    self.reveal_cursor.set(true);
                    self.open_detail(*c, cx)
                }
            }
            PaletteCmd::File(fi) => {
                if let Some(d) = self.detail.as_mut() {
                    if d.code.collapsed.remove(&fi) {
                        d.code.rebuild();
                    }
                    d.code.goto(fi);
                }
            }
        }
        cx.notify();
    }

    pub(crate) fn run_palette(&mut self, i: usize, cx: &mut Context<Self>) {
        self.run_palette_with(i, false, cx)
    }
}

impl GitViewer {
    pub(crate) fn render_palette(&self, cx: &mut Context<Self>) -> AnyElement {
        let q = self.palette_input.read(cx).text();
        let items: Vec<PaletteItem> = self
            .palette_entries(cx)
            .into_iter()
            .map(|(_, it)| it)
            .collect();
        let sel = self.nav.palette_sel.min(items.len().saturating_sub(1));
        let (prefix, rest) = split_prefix(&q);
        let palette = CommandPalette::new(self.palette_input.clone())
            .items(items)
            .selected(sel)
            .on_move(cx.listener(|this, d: &isize, _, cx| this.palette_move(*d, cx)))
            .on_hover(cx.listener(|this, i: &usize, _, cx| {
                if this.nav.palette_sel != *i {
                    this.nav.palette_sel = *i;
                    cx.notify()
                }
            }))
            .on_run(cx.listener(|this, i: &usize, window, cx| {
                this.run_palette(*i, cx);
                window.focus(&this.focus);
            }))
            .on_alt_run(cx.listener(|this, i: &usize, window, cx| {
                this.run_palette_with(*i, true, cx);
                window.focus(&this.focus);
            }))
            .on_close(cx.listener(|this, _: &(), window, cx| {
                this.overlay = Overlay::None;
                window.focus(&this.focus);
                cx.notify()
            }));
        let palette = if prefix == Prefix::None && rest.is_empty() {
            palette.footer(PREFIX_HINT)
        } else {
            palette
        };
        palette.into_any_element()
    }
}
