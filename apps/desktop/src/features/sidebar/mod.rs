//! Left sidebar: views, projects and settings entry points.

use gpui::{div, prelude::*, px, ClickEvent, Context};

use crate::app::{GitViewer, Overlay};
use crate::ui::atoms::{Button, ButtonVariant, NavItem, SectionLabel};
use crate::ui::theme;

impl GitViewer {
    pub(crate) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let add_project = Button::new("add-project", "").icon("+").variant(ButtonVariant::Ghost).on_click(cx.listener(
            |this, _: &ClickEvent, _, cx| {
                this.overlay = Overlay::AddProject;
                let n = this.selected.map(|_| this.project_name()).unwrap_or_default();
                this.add_name.update(cx, |i, cx| i.set_text(n, cx));
                cx.notify();
            },
        ));
        let mut col = div()
            .flex()
            .flex_col()
            .w(px(220.))
            .h_full()
            .flex_shrink_0()
            .bg(theme::panel(self.settings.panel_opacity))
            .border_r_1()
            .border_color(theme::border())
            .pt(px(44.))
            .px_2()
            .gap_0p5()
            .child(SectionLabel::new("Views"))
            .child(
                NavItem::new("nav-inbox", "Inbox")
                    .active(self.selected.is_none())
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.select_project(None, cx))),
            )
            .child(div().flex().justify_between().items_center().pr_1().child(SectionLabel::new("Projects")).child(add_project));
        for p in &self.projects {
            let id = p.id;
            col = col.child(
                NavItem::new(format!("nav-{id}"), p.name.clone())
                    .active(self.selected == Some(id))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select_project(Some(id), cx))),
            );
        }
        if self.projects.is_empty() {
            col = col.child(div().px_2().py_1().text_xs().text_color(theme::faint()).child("No projects yet"));
        }
        col.child(div().flex_1())
            .child(
                NavItem::new("nav-alias", "Repo aliases")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.open_alias(None, cx))),
            )
            .child(NavItem::new("nav-settings", "Appearance").active(self.overlay == Overlay::Settings).mb_2().on_click(
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.overlay = Overlay::Settings;
                    cx.notify()
                }),
            ))
    }
}
