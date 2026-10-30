//! Right-click menu of a list row: snooze, hide, star and links.

use gpui::{prelude::*, AnyElement, ClickEvent, Context, MouseDownEvent};
use gv_engine::{ChangeRequest, SnoozeKind};

use crate::app::GitViewer;
use crate::features::keymap::{self, CopyLink, OpenInBrowser, ToggleStar};
use crate::state::Copied;
use crate::ui::atoms::IconName;
use crate::ui::molecules::{ContextMenu, MenuItem, MenuSeparator};

impl GitViewer {
    /// Snooze and hide items, shared by the row menu and the detail "More" menu.
    pub(crate) fn snooze_items(&self, c: &ChangeRequest, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let item = |id: &str, label: &'static str, icon: IconName, kind: SnoozeKind, cx: &mut Context<Self>| {
            let cr = c.clone();
            MenuItem::new(format!("{id}-{}", c.id.key()), label).icon(icon).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.snooze(&cr, kind, cx))).into_any_element()
        };
        vec![
            item("snooze-week", "Snooze for 1 week", IconName::Clock, SnoozeKind::Week, cx),
            item("snooze-upd", "Snooze until updated", IconName::Clock, SnoozeKind::UntilUpdated, cx),
            item("hide-pr", "Hide this PR", IconName::EyeOff, SnoozeKind::Hidden, cx),
        ]
    }

    pub(super) fn render_row_menu(&self, c: &ChangeRequest, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (id, at) = self.quiet.row_menu.as_ref().filter(|(id, _)| *id == c.id)?;
        let (url, link, sid) = (c.web_url.clone(), c.web_url.clone(), id.clone());
        let starred = self.starred.contains(id);
        let close = |this: &mut Self| this.quiet.row_menu = None;
        let menu = ContextMenu::new(format!("row-ctx-{}", id.key()), *at)
            .on_dismiss(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                this.quiet.row_menu = None;
                cx.notify()
            }))
            .child(MenuItem::new("row-open", "Open in browser").icon(IconName::ExternalLink).shortcut(keymap::kbd::<OpenInBrowser>()).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                close(this);
                cx.open_url(&url)
            })))
            .child(MenuItem::new("row-copy", "Copy link").icon(IconName::Link).shortcut(keymap::kbd::<CopyLink>()).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                close(this);
                this.copy(Copied::Link, link.clone(), cx)
            })))
            .child(MenuItem::new("row-star", if starred { "Unstar" } else { "Star" }).icon(IconName::Star).shortcut(keymap::kbd::<ToggleStar>()).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                close(this);
                this.toggle_star(sid.clone(), cx)
            })))
            .child(MenuSeparator)
            .children(self.snooze_items(c, cx));
        Some(menu.into_any_element())
    }
}
