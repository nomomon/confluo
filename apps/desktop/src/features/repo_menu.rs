//! Repository name context menu: "Rename repository..." (a small field, Enter saves, "Reset to
//! default") and "Hide repository".

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, MouseDownEvent};

use crate::app::GitViewer;
use crate::ui::atoms::{Button, ButtonVariant, IconName};
use crate::ui::molecules::{ContextMenu, MenuItem};
use crate::ui::theme;

impl GitViewer {
    pub(crate) fn render_repo_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let m = self.repo_menu.clone()?;
        let menu = ContextMenu::new("repo-menu", m.pos).on_dismiss(cx.listener(
            |this, _: &MouseDownEvent, _, cx| {
                this.repo_menu = None;
                this.refocus = true;
                cx.notify()
            },
        ));
        let repo = m.repo.clone();
        if !m.renaming {
            return Some(
                menu.child(
                    MenuItem::new("repo-rename", "Rename repository...")
                        .icon(IconName::Pencil)
                        .on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| this.start_repo_rename(cx)),
                        ),
                )
                .child(
                    MenuItem::new("repo-hide", "Hide repository")
                        .icon(IconName::EyeOff)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.repo_menu = None;
                            this.set_repo_hidden(&repo, true, cx)
                        })),
                )
                .into_any_element(),
            );
        }
        let field = div()
            .w(theme::SPACE_16 * 4.)
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::focus_ring())
            .bg(theme::surface())
            .py(theme::SPACE_1)
            .child(self.alias_input.clone());
        let reset = Button::new("repo-reset", "Reset to default")
            .variant(ButtonVariant::Link)
            .on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| this.set_alias(&repo, None, cx)),
            );
        Some(
            menu.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_2)
                    .p(theme::SPACE_2)
                    .child(
                        div()
                            .text_size(theme::text_xs())
                            .text_color(theme::text_muted())
                            .child("Rename how this repository appears in Confluo"),
                    )
                    .child(field)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(reset)
                            .child(
                                div()
                                    .text_size(theme::text_xs())
                                    .text_color(theme::text_subtle())
                                    .child("Enter to save"),
                            ),
                    ),
            )
            .into_any_element(),
        )
    }
}
