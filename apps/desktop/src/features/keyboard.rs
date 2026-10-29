//! Window-level shortcuts: list navigation, dismissal and PR actions.
//!
//! Letter keys only fire outside text inputs. Keyboard moves are never animated.

use gpui::{actions, prelude::*, App, Context, Focusable, KeyBinding, Window};
use gv_engine::{ChangeRequest, Involvement};

use crate::app::{GitViewer, Overlay};
use crate::features::pr_detail::checkout_command;
use crate::state::Copied;

actions!(
    git_viewer,
    [SelectNext, SelectPrev, OpenSelected, Dismiss, FocusSearch, Refresh, OpenInBrowser, CopyCheckout, ToggleStar, TabAuthored, TabReviewing, TabAll, OpenSettings, About, Minimize, Zoom]
);

pub fn bind_keys(cx: &mut App) {
    let list = Some("GitViewer && !TextInput");
    let any = Some("GitViewer");
    cx.bind_keys([
        KeyBinding::new("j", SelectNext, list),
        KeyBinding::new("down", SelectNext, list),
        KeyBinding::new("k", SelectPrev, list),
        KeyBinding::new("up", SelectPrev, list),
        KeyBinding::new("enter", OpenSelected, list),
        KeyBinding::new("/", FocusSearch, list),
        KeyBinding::new("o", OpenInBrowser, list),
        KeyBinding::new("c", CopyCheckout, list),
        KeyBinding::new("s", ToggleStar, list),
        KeyBinding::new("1", TabAuthored, list),
        KeyBinding::new("2", TabReviewing, list),
        KeyBinding::new("3", TabAll, list),
        KeyBinding::new("escape", Dismiss, any),
        KeyBinding::new("cmd-r", Refresh, any),
        KeyBinding::new("cmd-,", OpenSettings, any),
    ]);
}

impl GitViewer {
    /// Attach the handlers to the root element.
    pub(crate) fn key_handlers<E: InteractiveElement>(&self, el: E, cx: &mut Context<Self>) -> E {
        el.key_context("GitViewer")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &SelectNext, _, cx| this.move_cursor(1, cx)))
            .on_action(cx.listener(|this, _: &SelectPrev, _, cx| this.move_cursor(-1, cx)))
            .on_action(cx.listener(|this, _: &OpenSelected, _, cx| {
                if let Some(c) = this.cursor_item(cx) {
                    this.open_detail(c, cx)
                }
            }))
            .on_action(cx.listener(Self::dismiss))
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| window.focus(&this.search.focus_handle(cx))))
            .on_action(cx.listener(|this, _: &Refresh, _, cx| this.refresh(cx)))
            .on_action(cx.listener(|this, _: &OpenInBrowser, _, cx| {
                if let Some(c) = this.target_item(cx) {
                    cx.open_url(&c.web_url)
                }
            }))
            .on_action(cx.listener(|this, _: &CopyCheckout, _, cx| {
                if let Some(c) = this.target_item(cx) {
                    this.copy(Copied::Checkout, checkout_command(&c.id), cx)
                }
            }))
            .on_action(cx.listener(|this, _: &ToggleStar, _, cx| {
                if let Some(c) = this.target_item(cx) {
                    this.toggle_star(c.id, cx)
                }
            }))
            .on_action(cx.listener(|this, _: &TabAuthored, _, cx| this.set_tab(Involvement::Authored, cx)))
            .on_action(cx.listener(|this, _: &TabReviewing, _, cx| this.set_tab(Involvement::Reviewing, cx)))
            .on_action(cx.listener(|this, _: &TabAll, _, cx| this.set_tab(Involvement::All, cx)))
            .on_action(cx.listener(|_, _: &Minimize, window, _| window.minimize_window()))
            .on_action(cx.listener(|_, _: &Zoom, window, _| window.zoom_window()))
            .on_action(cx.listener(|_, _: &About, window, cx| {
                let detail = format!("Version {}\nPRs and MRs across GitHub, GitLab and Azure DevOps.", env!("CARGO_PKG_VERSION"));
                let _ = window.prompt(gpui::PromptLevel::Info, "Confluo", Some(&detail), &["OK"], cx);
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| {
                this.overlay = Overlay::Settings;
                cx.notify()
            }))
    }

    fn cursor_item(&self, cx: &App) -> Option<ChangeRequest> {
        let id = self.cursor.as_ref()?;
        self.items.iter().find(|c| &c.id == id).cloned().filter(|_| self.nav_order(cx).contains(id))
    }

    /// The PR a shortcut acts on: the open detail, else the keyboard selection.
    fn target_item(&self, cx: &App) -> Option<ChangeRequest> {
        self.detail.as_ref().map(|d| d.summary.clone()).or_else(|| self.cursor_item(cx))
    }

    fn move_cursor(&mut self, delta: isize, cx: &mut Context<Self>) {
        let order = self.nav_order(cx);
        if order.is_empty() {
            return;
        }
        let current = self.cursor.clone().or_else(|| self.detail.as_ref().map(|d| d.summary.id.clone()));
        let next = match current.and_then(|id| order.iter().position(|x| *x == id)) {
            Some(i) => (i as isize + delta).clamp(0, order.len() as isize - 1) as usize,
            None if delta > 0 => 0,
            None => order.len() - 1,
        };
        self.set_cursor(order[next].clone(), cx);
        self.reveal_cursor.set(true);
        if self.detail.is_some() {
            if let Some(c) = self.cursor_item(cx) {
                self.open_detail(c, cx);
            }
        }
        cx.notify();
    }

    /// Escape closes the topmost thing: menu, dialog, detail, then the search text.
    fn dismiss(&mut self, _: &Dismiss, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.is_some() {
            self.menu = None;
        } else if self.overlay != Overlay::None {
            self.overlay = Overlay::None;
        } else if self.search.focus_handle(cx).is_focused(window) || self.detail.is_none() {
            self.search.update(cx, |i, cx| i.set_text("", cx));
        } else {
            self.close_detail(cx);
        }
        window.focus(&self.focus);
        cx.notify();
    }

    fn set_tab(&mut self, tab: Involvement, cx: &mut Context<Self>) {
        self.tab = tab;
        self.reload();
        cx.notify();
    }
}
