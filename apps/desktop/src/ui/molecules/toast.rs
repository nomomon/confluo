//! Sonner-like toaster: a stack of cards in the bottom-right corner of its (relative) parent,
//! newest at the bottom.
//!
//! `Toaster::new().toast(Toast::new("Copied").description("Link copied").variant(ToastVariant::Success))`

use gpui::{div, prelude::*, px, App, SharedString, Window};

use crate::ui::atoms::{Icon, IconName};
use crate::ui::{motion, theme};

#[allow(dead_code)] // kit API
#[derive(Clone, Copy, PartialEq, Default)]
pub enum ToastVariant {
    #[default]
    Default,
    Success,
    Error,
}

#[derive(IntoElement)]
pub struct Toast {
    title: SharedString,
    description: Option<SharedString>,
    variant: ToastVariant,
}

impl Toast {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self { title: title.into(), description: None, variant: ToastVariant::default() }
    }

    #[allow(dead_code)] // kit API
    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn variant(mut self, v: ToastVariant) -> Self {
        self.variant = v;
        self
    }
}

impl RenderOnce for Toast {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let icon = match self.variant {
            ToastVariant::Default => None,
            ToastVariant::Success => Some(Icon::new(IconName::CircleCheck).color(theme::success())),
            ToastVariant::Error => Some(Icon::new(IconName::AlertCircle).color(theme::danger())),
        };
        div()
            .w(px(320.))
            .flex()
            .items_start()
            .gap(theme::SPACE_2P5)
            .px(theme::SPACE_4)
            .py(theme::SPACE_3)
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_lg()
            .text_size(theme::text_sm())
            .children(icon.map(|i| div().pt(px(2.)).child(i)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_0P5)
                    .child(div().font_weight(theme::WEIGHT_MEDIUM).child(self.title))
                    .children(self.description.map(|d| div().text_size(theme::text_xs()).text_color(theme::text_muted()).child(d))),
            )
    }
}

#[derive(IntoElement, Default)]
pub struct Toaster {
    toasts: Vec<Toast>,
}

impl Toaster {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toast(mut self, t: Toast) -> Self {
        self.toasts.push(t);
        self
    }
}

impl RenderOnce for Toaster {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let n = self.toasts.len();
        div()
            .absolute()
            .bottom(theme::SPACE_6)
            .right(theme::SPACE_6)
            .flex()
            .flex_col()
            .items_end()
            .gap(theme::SPACE_2)
            .children(self.toasts.into_iter().enumerate().map(|(i, t)| if i + 1 == n { motion::pop_in(div().child(t), ("toast", n)) } else { t.into_any_element() }))
    }
}
