//! Project picker dropdown opened from the list header.

use gpui::{prelude::*, px, ClickEvent, Context};

use crate::app::GitViewer;
use crate::ui::atoms::NavItem;
use crate::ui::molecules::DropdownMenu;

impl GitViewer {
    pub(crate) fn render_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut menu = DropdownMenu::new().top(px(70.)).right(px(110.)).child(
            NavItem::new("pick-inbox", "Inbox")
                .active(self.selected.is_none())
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.select_project(None, cx))),
        );
        for p in &self.projects {
            let id = p.id;
            menu = menu.child(
                NavItem::new(format!("pick-{id}"), p.name.clone())
                    .active(self.selected == Some(id))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select_project(Some(id), cx))),
            );
        }
        menu
    }
}
