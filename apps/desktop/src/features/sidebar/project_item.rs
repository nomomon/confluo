//! One project in the sidebar, with a hover kebab and right-click menu.

use gpui::{div, prelude::*, ClickEvent, Context, MouseDownEvent};
use gv_engine::Project;

use crate::app::{GitViewer, Layer};
use crate::ui::atoms::{tooltip, Icon, IconName, NavItem};
use crate::ui::molecules::{ContextMenu, DropdownMenu, MenuItem, MenuLabel, MenuSeparator};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_project_item(
        &self,
        i: usize,
        p: &Project,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = p.id;
        let kebab = div()
            .id(gpui::SharedString::from(format!("project-more-{id}")))
            .px(theme::space_1())
            .rounded(theme::RADIUS_SM)
            .text_color(theme::text_subtle())
            .hover(|s| s.text_color(theme::text()).bg(theme::surface_hover()))
            .tooltip(tooltip("More"))
            .child(
                Icon::new(IconName::Ellipsis)
                    .size(theme::icon_sm())
                    .color(theme::text_subtle()),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.toggle_project_menu(id, cx)
            }));
        let item = NavItem::new(format!("nav-{id}"), p.name.clone())
            .icon(IconName::Folder)
            .compact(self.compact)
            .active(self.selected == Some(id))
            .trailing_on_hover(kebab)
            .on_right_click(cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                this.open_layer(Layer::ProjectAt(id, ev.position), cx)
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.starred_only = false;
                this.select_project(Some(id), cx)
            }));
        let dismiss = || {
            cx.listener(
                |this: &mut Self, _: &MouseDownEvent, _, cx: &mut Context<Self>| {
                    this.dismiss_layer(cx)
                },
            )
        };
        let items = |cx: &mut Context<Self>| {
            [
                MenuItem::new(format!("rename-{id}"), "Rename")
                    .icon(IconName::Pencil)
                    .on_click(
                        cx.listener(move |this, _: &ClickEvent, _, cx| this.open_rename(id, cx)),
                    )
                    .into_any_element(),
                MenuItem::new(format!("add-src-{id}"), "Add source")
                    .icon(IconName::Plus)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.layer = None;
                        this.add_source_to(id, cx)
                    }))
                    .into_any_element(),
                MenuSeparator.into_any_element(),
                MenuItem::new(format!("delete-{id}"), "Delete")
                    .icon(IconName::Trash)
                    .destructive()
                    .on_click(
                        cx.listener(move |this, _: &ClickEvent, _, cx| this.confirm_delete(id, cx)),
                    )
                    .into_any_element(),
            ]
        };
        let menu = match self.layer.clone() {
            Some(Layer::Project(m)) if m == id => Some(
                DropdownMenu::new(format!("project-menu-{id}"))
                    .on_dismiss(dismiss())
                    .children(items(cx))
                    .into_any_element(),
            ),
            Some(Layer::ProjectAt(m, at)) if m == id => Some(
                ContextMenu::new(format!("project-ctx-{id}"), at)
                    .on_dismiss(dismiss())
                    .child(MenuLabel::new(p.name.clone()))
                    .children(items(cx))
                    .into_any_element(),
            ),
            _ => None,
        };
        let hint = self.project_hint(i).map(|h| {
            if self.compact {
                h.anchor(crate::ui::atoms::HintAnchor::Corner)
            } else {
                h
            }
        });
        div().relative().child(item).children(hint).children(menu)
    }

    fn toggle_project_menu(&mut self, id: i64, cx: &mut Context<Self>) {
        self.toggle_layer(Layer::Project(id), cx);
    }
}
