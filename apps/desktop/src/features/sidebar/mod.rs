//! Left sidebar: Inbox, Starred, projects (with rename and delete) and settings.

mod project_item;

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::{GitViewer, Overlay};
use crate::features::shared::layout;
use crate::ui::atoms::{Button, ButtonVariant, IconName, NavItem, SectionLabel};
use crate::ui::theme;

impl GitViewer {
    pub(crate) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let add_project = Button::new("add-project", "")
            .icon(IconName::Plus)
            .variant(ButtonVariant::Ghost)
            .tooltip("Add project")
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.open_add_project(cx)));
        let projects_header = div()
            .group("projects-header")
            .flex()
            .justify_between()
            .items_center()
            .pr(theme::SPACE_1)
            .child(SectionLabel::new("Projects"))
            .child(div().opacity(0.).group_hover("projects-header", |s| s.opacity(1.)).child(add_project));
        let in_inbox = self.selected.is_none();
        let mut col = div()
            .flex()
            .flex_col()
            .w(layout::SIDEBAR_W)
            .h_full()
            .flex_shrink_0()
            .bg(self.surfaces().panel())
            .border_r_1()
            .border_color(theme::border_subtle())
            .pt(layout::SIDEBAR_TOP)
            .px(theme::SPACE_3)
            .gap(theme::SPACE_1)
            .child(NavItem::new("nav-inbox", "Inbox").icon(IconName::Inbox).active(in_inbox && !self.starred_only).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.starred_only = false;
                this.select_project(None, cx)
            })))
            .child(NavItem::new("nav-starred", "Starred").icon(IconName::Star).active(in_inbox && self.starred_only).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.starred_only = true;
                this.select_project(None, cx)
            })))
            .child(projects_header);
        for p in &self.projects {
            col = col.child(self.render_project_item(p, cx));
        }
        if self.projects.is_empty() {
            col = col.child(div().px(theme::SPACE_3).py(theme::SPACE_2).text_size(theme::text_xs()).text_color(theme::text_subtle()).child("No projects yet"));
        }
        col.child(div().flex_1()).child(NavItem::new("nav-settings", "Settings").icon(IconName::Settings).active(self.overlay == Overlay::Settings).mb(theme::SPACE_4).on_click(cx.listener(
            |this, _: &ClickEvent, _, cx| {
                this.overlay = Overlay::Settings;
                cx.notify()
            },
        )))
    }
}
