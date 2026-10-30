//! Command palette: fuzzy search over PRs and app commands, running the chosen one, and its overlay.

use gpui::{AnyElement, Context, Focusable, IntoElement, SharedString};
use gv_engine::ChangeRequest;

use crate::app::{GitViewer, Overlay};
use crate::features::keymap::{self, GoInbox, GoStarred, OpenSettings, Refresh, ToggleDetail, ToggleSidebar, ToggleTheme};
use crate::ui::atoms::IconName;
use crate::ui::molecules::command_palette::score;
use crate::ui::molecules::{CommandPalette, PaletteItem};

/// What a palette row does.
#[derive(Clone)]
pub enum PaletteCmd {
    Open(Box<ChangeRequest>),
    Refresh,
    Settings,
    ToggleTheme,
    ToggleSidebar,
    ToggleDetail,
    Inbox,
    Starred,
    Project(i64),
}

impl GitViewer {
    /// Rows for the current query: commands and projects, then PRs, best match first.
    pub(crate) fn palette_entries(&self, cx: &gpui::App) -> Vec<(PaletteCmd, PaletteItem)> {
        let q = self.palette_input.read(cx).text();
        let item = |icon, title: String, detail: Option<String>, shortcut: String, group| PaletteItem {
            icon,
            title: title.into(),
            detail: detail.map(SharedString::from),
            shortcut: (!shortcut.is_empty()).then(|| shortcut.into()),
            group,
        };
        let mut cmds: Vec<(PaletteCmd, PaletteItem)> = vec![
            (PaletteCmd::Refresh, item(IconName::Refresh, "Refresh".into(), None, keymap::kbd::<Refresh>(), "Actions")),
            (PaletteCmd::Settings, item(IconName::Settings, "Settings".into(), None, keymap::kbd::<OpenSettings>(), "Actions")),
            (PaletteCmd::ToggleTheme, item(IconName::Moon, "Toggle theme".into(), None, keymap::kbd::<ToggleTheme>(), "Actions")),
            (PaletteCmd::ToggleSidebar, item(IconName::LayoutSidebar, "Toggle sidebar".into(), None, keymap::kbd::<ToggleSidebar>(), "Actions")),
            (PaletteCmd::ToggleDetail, item(IconName::LayoutColumns, "Toggle detail pane".into(), None, keymap::kbd::<ToggleDetail>(), "Actions")),
            (PaletteCmd::Inbox, item(IconName::Inbox, "Go to Inbox".into(), None, keymap::kbd::<GoInbox>(), "Go to")),
            (PaletteCmd::Starred, item(IconName::Star, "Go to Starred".into(), None, keymap::kbd::<GoStarred>(), "Go to")),
        ];
        cmds.extend(self.projects.iter().map(|p| (PaletteCmd::Project(p.id), item(IconName::Folder, format!("Go to {}", p.name), None, String::new(), "Go to"))));
        let prs = self.items.iter().chain(&self.quiet.stale).map(|c| {
            let detail = format!("{} {}", self.repo_label(&c.id.repo), crate::features::shared::number_label(c));
            (PaletteCmd::Open(Box::new(c.clone())), item(crate::features::shared::state_icon_name(c.state), c.title.clone(), Some(detail), String::new(), "Pull requests"))
        });
        if q.trim().is_empty() {
            let mut v = cmds;
            v.extend(prs.take(20));
            return v;
        }
        let haystack = |cmd: &PaletteCmd, it: &PaletteItem| match cmd {
            PaletteCmd::Open(c) => format!("{} {} {} #{}", c.title, c.id.repo.path, self.repo_label(&c.id.repo), c.id.number),
            _ => it.title.to_string(),
        };
        let mut scored: Vec<(i32, usize, (PaletteCmd, PaletteItem))> = cmds
            .into_iter()
            .chain(prs)
            .enumerate()
            .filter_map(|(i, (cmd, it))| {
                let num = q.trim_start_matches(['#', '!']);
                let exact = matches!(&cmd, PaletteCmd::Open(c) if c.id.number.to_string() == num);
                let s = if exact { Some(i32::MAX) } else { score(&q, &haystack(&cmd, &it)) };
                s.map(|s| (s, i, (cmd, it)))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        // Keep group headings contiguous: commands first, then PRs, each by score.
        let (mut a, b): (Vec<_>, Vec<_>) = scored.into_iter().map(|x| x.2).partition(|(c, _)| !matches!(c, PaletteCmd::Open(_)));
        a.extend(b);
        a
    }

    pub(crate) fn open_palette(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        if self.overlay == Overlay::Palette {
            self.overlay = Overlay::None;
            window.focus(&self.focus);
        } else {
            self.menu = None;
            self.overlay = Overlay::Palette;
            self.nav.palette_sel = 0;
            self.palette_input.update(cx, |i, cx| i.set_text("", cx));
            window.focus(&self.palette_input.read(cx).focus_handle(cx));
        }
        cx.notify();
    }

    pub(crate) fn palette_move(&mut self, delta: isize, cx: &mut Context<Self>) {
        let n = self.palette_entries(cx).len().min(50);
        if n > 0 {
            self.nav.palette_sel = (self.nav.palette_sel as isize + delta).rem_euclid(n as isize) as usize;
        }
        cx.notify();
    }

    /// Run row `i`; the window refocuses the root on the next Dismiss/close.
    pub(crate) fn run_palette(&mut self, i: usize, cx: &mut Context<Self>) {
        let Some((cmd, _)) = self.palette_entries(cx).into_iter().nth(i) else { return };
        self.overlay = Overlay::None;
        self.refocus = true;
        match cmd {
            PaletteCmd::Open(c) => {
                self.set_cursor(c.id.clone(), cx);
                self.reveal_cursor.set(true);
                self.open_detail(*c, cx)
            }
            PaletteCmd::Refresh => self.refresh(cx),
            PaletteCmd::Settings => self.open_settings(None, cx),
            PaletteCmd::ToggleTheme => self.toggle_theme(cx),
            PaletteCmd::ToggleSidebar => self.toggle_sidebar(cx),
            PaletteCmd::ToggleDetail => self.toggle_detail(cx),
            PaletteCmd::Inbox => self.go_view(1, cx),
            PaletteCmd::Starred => self.go_view(2, cx),
            PaletteCmd::Project(id) => {
                self.starred_only = false;
                self.select_project(Some(id), cx)
            }
        }
        cx.notify();
    }
}

impl GitViewer {
    pub(crate) fn render_palette(&self, cx: &mut Context<Self>) -> AnyElement {
        let items: Vec<PaletteItem> = self.palette_entries(cx).into_iter().map(|(_, it)| it).collect();
        let sel = self.nav.palette_sel.min(items.len().saturating_sub(1));
        CommandPalette::new(self.palette_input.clone())
            .items(items)
            .selected(sel)
            .on_move(cx.listener(|this, d: &isize, _, cx| this.palette_move(*d, cx)))
            .on_run(cx.listener(|this, i: &usize, window, cx| {
                this.run_palette(*i, cx);
                window.focus(&this.focus);
            }))
            .on_close(cx.listener(|this, _: &(), window, cx| {
                this.overlay = Overlay::None;
                window.focus(&this.focus);
                cx.notify()
            }))
            .into_any_element()
    }
}
