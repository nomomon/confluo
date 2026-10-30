//! Left sidebar: Inbox, Starred, projects (with rename and delete) and settings.

mod project_item;
mod update_card;

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::{GitViewer, Overlay};
use crate::features::shared::layout;
use crate::features::keymap::{GoInbox, GoStarred, OpenSettings, ToggleSidebar};
use crate::ui::atoms::{Button, ButtonVariant, HintAnchor, IconName, NavItem, SectionLabel};
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
        let compact = self.compact;
        let anchor = if compact { HintAnchor::Corner } else { HintAnchor::Inline };
        let mut col = div()
            .flex()
            .flex_col()
            .w(self.sidebar_w())
            .h_full()
            .flex_shrink_0()
            .bg(self.surfaces().sidebar())
            .border_color(theme::border_subtle())
            .map(|c| if theme::sidebar_inset() { c.border_1().rounded(theme::RADIUS_XL).shadow_sm() } else { c.border_r_1() })
            .pt(layout::SIDEBAR_TOP)
            .px(theme::SPACE_3)
            .gap(theme::SPACE_1)
            .relative()
            .child(div().absolute().top_0().left_0().right_0().h(layout::SIDEBAR_TOP).children(self.hint::<ToggleSidebar>(HintAnchor::Inline)))
            .child(self.hinted::<GoInbox>(NavItem::new("nav-inbox", "Inbox").icon(IconName::Inbox).compact(compact).active(in_inbox && !self.starred_only).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.starred_only = false;
                this.select_project(None, cx)
            })), anchor))
            .child(self.hinted::<GoStarred>(NavItem::new("nav-starred", "Starred").icon(IconName::Star).compact(compact).active(in_inbox && self.starred_only).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.starred_only = true;
                this.select_project(None, cx)
            })), anchor))
            .child(if compact { div().my(theme::SPACE_3).mx(theme::SPACE_2).h(gpui::px(1.)).bg(theme::border_subtle()).into_any_element() } else { projects_header.into_any_element() });
        for (i, p) in self.projects.iter().enumerate() {
            col = col.child(self.render_project_item(i, p, cx));
        }
        if self.projects.is_empty() && !compact {
            col = col.child(div().px(theme::SPACE_3).py(theme::SPACE_2).text_size(theme::text_xs()).text_color(theme::text_subtle()).child("No projects yet"));
        }
        let settings = NavItem::new("nav-settings", "Settings").icon(IconName::Settings).compact(compact).active(self.overlay == Overlay::Settings).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.overlay = Overlay::Settings;
            cx.notify()
        }));
        let col = col.child(div().flex_1()).children(self.render_update_card(cx)).child(self.hinted::<OpenSettings>(settings, anchor).mb(theme::SPACE_4));
        // macOS 26 light: the sidebar floats as an inset glass panel; its width includes the margin.
        let inset = theme::sidebar_inset();
        div().h_full().flex_shrink_0().when(inset, |d| d.p(layout::SIDEBAR_INSET)).child(col.when(inset, |c| c.w(self.sidebar_w() - layout::SIDEBAR_INSET * 2.).pt(layout::SIDEBAR_TOP - layout::SIDEBAR_INSET)))
    }
}
