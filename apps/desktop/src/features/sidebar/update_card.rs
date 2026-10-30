//! "Update to vX.Y.Z" card at the bottom of the sidebar: install, release notes, dismiss.

use gpui::{div, prelude::*, relative, AnyElement, ClickEvent, Context};

use crate::app::GitViewer;
use crate::ui::atoms::{Button, ButtonVariant, IconName};
use crate::ui::molecules::Card;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_update_card(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let r = self.update.available.as_ref()?;
        if self.compact {
            let b = Button::new("update-rail", "")
                .icon(IconName::Download)
                .variant(ButtonVariant::Ghost)
                .tooltip("Update Confluo");
            return Some(
                b.on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.install_update(cx)))
                    .into_any_element(),
            );
        }
        let title = format!("Update to v{}", r.version());
        let body = match self.update.progress {
            Some(p) => {
                let bar = div()
                    .h(theme::space_1())
                    .w_full()
                    .rounded(theme::RADIUS_FULL)
                    .bg(theme::surface_hover())
                    .child(
                        div()
                            .h_full()
                            .w(relative(p.clamp(0., 1.)))
                            .rounded(theme::RADIUS_FULL)
                            .bg(theme::accent()),
                    );
                let label = div()
                    .text_size(theme::text_xs())
                    .text_color(theme::text_muted())
                    .child(format!("Downloading {}%", (p * 100.).round() as u32));
                div()
                    .flex()
                    .flex_col()
                    .gap(theme::space_1p5())
                    .child(bar)
                    .child(label)
                    .into_any_element()
            }
            None => {
                let url = r.html_url.clone();
                let install = Button::new("update-install", "Update")
                    .variant(ButtonVariant::Primary)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.install_update(cx)));
                let notes = Button::new("update-notes", "What's new")
                    .variant(ButtonVariant::Link)
                    .on_click(move |_, _, cx| cx.open_url(&url));
                let dismiss = Button::new("update-dismiss", "")
                    .icon(IconName::X)
                    .variant(ButtonVariant::Ghost)
                    .tooltip("Dismiss this version")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.dismiss_update(cx)));
                div()
                    .flex()
                    .items_center()
                    .gap(theme::space_2())
                    .child(install)
                    .child(notes)
                    .child(div().flex_1())
                    .child(dismiss)
                    .into_any_element()
            }
        };
        Some(
            div()
                .mb(theme::space_2())
                .child(
                    Card::new(self.surfaces())
                        .title(title)
                        .description("A new version of Confluo is ready")
                        .child(body),
                )
                .into_any_element(),
        )
    }
}
