//! Left sidebar: views, projects and settings entry points.

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::{GitViewer, Overlay};
use crate::ui::atoms::{Button, ButtonVariant, NavItem, SectionLabel};
use crate::ui::theme;
use crate::features::shared::layout;

impl GitViewer {
    pub(crate) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let add_project = Button::new("add-project", "").icon("+").variant(ButtonVariant::Ghost).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.open_add_project(cx)));
        let mut col = div()
            .flex()
            .flex_col()
            .w(layout::SIDEBAR_W)
            .h_full()
            .flex_shrink_0()
            .bg(self.surfaces().panel())
            .border_r_1()
            .border_color(theme::BORDER_SUBTLE)
            .pt(layout::SIDEBAR_TOP)
            .px(theme::SPACE_2)
            .gap(theme::SPACE_0P5)
            .child(SectionLabel::new("Views"))
            .child(
                NavItem::new("nav-inbox", "Inbox")
                    .active(self.selected.is_none())
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.select_project(None, cx))),
            )
            .child(div().flex().justify_between().items_center().pr(theme::SPACE_1).child(SectionLabel::new("Projects")).child(add_project));
        for p in &self.projects {
            let id = p.id;
            col = col.child(
                NavItem::new(format!("nav-{id}"), p.name.clone())
                    .active(self.selected == Some(id))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select_project(Some(id), cx))),
            );
        }
        if self.projects.is_empty() {
            col = col.child(div().px(theme::SPACE_2).py(theme::SPACE_1).text_size(theme::TEXT_XS).text_color(theme::TEXT_SUBTLE).child("No projects yet"));
        }
        col.child(div().flex_1())
            .child(
                NavItem::new("nav-alias", "Repo aliases")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.open_alias(None, cx))),
            )
            .child(NavItem::new("nav-settings", "Appearance").active(self.overlay == Overlay::Settings).mb(theme::SPACE_2).on_click(
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.overlay = Overlay::Settings;
                    cx.notify()
                }),
            ))
    }
}
