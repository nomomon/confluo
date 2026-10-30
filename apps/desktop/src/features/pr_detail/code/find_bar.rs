//! Compact find bar over the diff (Cmd+F in Review mode): query, "3 of 12", case toggle,
//! previous / next and close. Enter and Shift+Enter step, Esc closes (`Dismiss`).

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};

use crate::app::GitViewer;
use crate::ui::atoms::{Button, ButtonVariant, IconName, Size};
use crate::ui::molecules::Input;
use crate::ui::{motion, theme};

/// Width of the bar's query field.
const FIND_INPUT_W: gpui::Pixels = gpui::px(240.);

impl GitViewer {
    pub(super) fn render_find_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.find.open {
            return None;
        }
        let q = self.find_input.read(cx).text();
        let label = self.find.count_label(&q);
        let none = !q.is_empty() && self.find.matches.is_empty();
        let icon = |id: &'static str, icon: IconName, tip: &str| {
            Button::new(id, "")
                .icon(icon)
                .variant(ButtonVariant::Ghost)
                .size(Size::Sm)
                .tooltip(tip.to_string())
        };
        let field = div().w(FIND_INPUT_W).child(
            Input::new("find-input", self.find_input.clone(), !q.is_empty())
                .leading(IconName::Search)
                .on_clear(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.find_input.update(cx, |i, cx| i.set_text("", cx))
                })),
        );
        let bar = div()
            .id("find-bar")
            .key_context("FindBar")
            .flex()
            .items_center()
            .gap(theme::space_1())
            .p(theme::space_1())
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_lg()
            .occlude()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(field)
            .child(
                div()
                    .min_w(theme::space_16())
                    .px(theme::space_1())
                    .text_size(theme::text_xs())
                    .text_color(if none {
                        theme::danger()
                    } else {
                        theme::text_muted()
                    })
                    .child(label),
            )
            .child(
                Button::new("find-case", "Aa")
                    .variant(if self.find.case {
                        ButtonVariant::Outline
                    } else {
                        ButtonVariant::Ghost
                    })
                    .size(Size::Sm)
                    .tooltip("Match case")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_find_case(cx))),
            )
            .child(
                icon(
                    "find-prev",
                    IconName::ChevronUp,
                    "Previous match  \u{21e7} \u{21a9}",
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.find_step(-1, cx))),
            )
            .child(
                icon("find-next", IconName::ChevronDown, "Next match  \u{21a9}")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.find_step(1, cx))),
            )
            .child(icon("find-close", IconName::X, "Close  esc").on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| this.close_find(window, cx)),
            ));
        Some(
            div()
                .absolute()
                .top(theme::space_2())
                .right(theme::space_4())
                .child(motion::pop_in(bar, "find-bar"))
                .into_any_element(),
        )
    }
}
