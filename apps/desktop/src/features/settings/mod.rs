//! Settings dialog, laid out like macOS System Settings: a left navigation of pages and grouped
//! rows on the right (label and description left, control right) inside rounded cards.
//! Every change applies live and is saved to settings.json.

mod accounts;
mod appearance;
mod filters;
mod general;
mod projects;
mod repositories;
mod shortcuts;

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};

use crate::app::{GitViewer, Overlay};
use crate::features::layout;
use crate::state::SettingsPage;
use crate::ui::atoms::{Button, ButtonVariant, IconName, NavItem};
use crate::ui::molecules::{Card, Dialog};
use crate::ui::theme;

fn page_icon(p: SettingsPage) -> IconName {
    match p {
        SettingsPage::General => IconName::AdjustmentsHorizontal,
        SettingsPage::Appearance => IconName::Brush,
        SettingsPage::Accounts => IconName::User,
        SettingsPage::Projects => IconName::Folder,
        SettingsPage::Repositories => IconName::Code,
        SettingsPage::Filters => IconName::Filter,
        SettingsPage::Shortcuts => IconName::Keyboard,
    }
}

impl GitViewer {
    pub(crate) fn render_settings_dialog(&self, cx: &mut Context<Self>) -> AnyElement {
        let nav = SettingsPage::ALL.iter().fold(
            div()
                .w(layout::SETTINGS_NAV_W)
                .flex_shrink_0()
                .flex()
                .flex_col()
                .gap(theme::SPACE_0P5)
                .p(theme::SPACE_3)
                .bg(theme::surface())
                .border_r_1()
                .border_color(theme::border_subtle())
                .child(
                    div()
                        .px(theme::SPACE_3)
                        .pt(theme::SPACE_2)
                        .pb(theme::SPACE_3)
                        .text_size(theme::text_sm())
                        .font_weight(theme::WEIGHT_SEMIBOLD)
                        .child("Settings"),
                ),
            |nav, p| {
                let p = *p;
                nav.child(
                    NavItem::new(format!("settings-nav-{}", p.label()), p.label())
                        .icon(page_icon(p))
                        .active(self.settings_page == p)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.settings_page = p;
                            cx.notify()
                        })),
                )
            },
        );
        let body: AnyElement = match self.settings_page {
            SettingsPage::General => self.render_general(cx).into_any_element(),
            SettingsPage::Appearance => self.render_appearance(cx).into_any_element(),
            SettingsPage::Accounts => self.render_accounts(cx).into_any_element(),
            SettingsPage::Projects => self.render_projects_page(cx).into_any_element(),
            SettingsPage::Repositories => self.render_repositories(cx).into_any_element(),
            SettingsPage::Filters => self.render_filters(cx).into_any_element(),
            SettingsPage::Shortcuts => self.render_shortcuts().into_any_element(),
        };
        let close = Button::new("settings-close", "")
            .icon(IconName::X)
            .variant(ButtonVariant::Ghost)
            .tooltip("Close  esc")
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.overlay = Overlay::None;
                cx.notify()
            }));
        let header = div()
            .flex()
            .items_center()
            .px(theme::SPACE_6)
            .pt(theme::SPACE_5)
            .pb(theme::SPACE_3)
            .child(
                div()
                    .flex_1()
                    .text_size(theme::text_base())
                    .font_weight(theme::WEIGHT_SEMIBOLD)
                    .child(self.settings_page.label()),
            )
            .child(close);
        let content = div()
            .flex_1()
            .min_w(theme::SPACE_0)
            .flex()
            .flex_col()
            .child(header)
            .child({
                let h = crate::ui::elastic::handle("settings-body");
                let pane = div()
                    .id("settings-body")
                    .track_scroll(&h)
                    .flex_1()
                    .min_h(theme::SPACE_0)
                    .map(crate::ui::scroll::y_only)
                    .flex()
                    .flex_col()
                    .px(theme::SPACE_6)
                    .pb(theme::SPACE_6);
                let pane = pane.child(
                    crate::ui::elastic::content("settings-body")
                        .w(layout::SETTINGS_W - layout::SETTINGS_NAV_W - theme::SPACE_6 * 2.)
                        .flex()
                        .flex_col()
                        .child(body),
                );
                crate::ui::elastic::y_scroller(pane, "settings-body", h)
            });
        Dialog::bare()
            .size(layout::SETTINGS_W, layout::SETTINGS_H)
            .content(div().flex().size_full().child(nav).child(content))
            .into_any_element()
    }

    /// A titled card of divided rows; the building block of every page.
    pub(super) fn settings_group(
        &self,
        title: Option<&'static str>,
        rows: Vec<AnyElement>,
    ) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(theme::SPACE_2)
            .pt(theme::SPACE_4)
            .children(title.map(|t| {
                div()
                    .px(theme::SPACE_1)
                    .text_size(theme::text_xs())
                    .font_weight(theme::WEIGHT_MEDIUM)
                    .text_color(theme::text_subtle())
                    .child(t)
            }))
            .child(Card::new(self.surfaces()).rows().children(rows))
    }
}
