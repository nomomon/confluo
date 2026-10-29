//! One project in the sidebar, with a hover kebab and right-click menu.

use gpui::{div, prelude::*, ClickEvent, Context, MouseDownEvent};
use gv_engine::Project;

use crate::app::{GitViewer, Menu};
use crate::ui::atoms::{tooltip, Icon, IconName, NavItem};
use crate::ui::molecules::DropdownMenu;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_project_item(&self, p: &Project, cx: &mut Context<Self>) -> impl IntoElement {
        let id = p.id;
        let kebab = div()
            .id(gpui::SharedString::from(format!("project-more-{id}")))
            .px(theme::SPACE_1)
            .rounded(theme::RADIUS_SM)
            .text_color(theme::TEXT_SUBTLE)
            .hover(|s| s.text_color(theme::TEXT).bg(theme::SURFACE_HOVER))
            .tooltip(tooltip("More"))
            .child(Icon::new(IconName::Ellipsis).size(theme::ICON_SM).color(theme::TEXT_SUBTLE))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.toggle_project_menu(id, cx)
            }));
        let item = NavItem::new(format!("nav-{id}"), p.name.clone())
            .icon(IconName::Folder)
            .active(self.selected == Some(id))
            .trailing_on_hover(kebab)
            .on_right_click(cx.listener(move |this, _: &MouseDownEvent, _, cx| this.toggle_project_menu(id, cx)))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.starred_only = false;
                this.select_project(Some(id), cx)
            }));
        let menu = (self.menu == Some(Menu::Project(id))).then(|| {
            DropdownMenu::new(format!("project-menu-{id}"))
                .top(theme::SPACE_8)
                .right(theme::SPACE_0)
                .on_dismiss(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.menu = None;
                    cx.notify()
                }))
                .child(NavItem::new(format!("rename-{id}"), "Rename").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_rename(id, cx))))
                .child(NavItem::new(format!("delete-{id}"), "Delete").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.confirm_delete(id, cx))))
        });
        div().relative().child(item).children(menu)
    }

    fn toggle_project_menu(&mut self, id: i64, cx: &mut Context<Self>) {
        self.menu = if self.menu == Some(Menu::Project(id)) { None } else { Some(Menu::Project(id)) };
        cx.notify();
    }
}
