//! Keyboard shortcut hint like shadcn `Kbd`: each key in a small raised box.
//!
//! `Kbd::new("⌘ R")` (space separated keys)

use gpui::{div, prelude::*, px, App, SharedString, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct Kbd(SharedString);

impl Kbd {
    pub fn new(keys: impl Into<SharedString>) -> Self {
        Self(keys.into())
    }
}

impl RenderOnce for Kbd {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().flex().flex_shrink_0().gap(theme::SPACE_0P5).children(self.0.split_whitespace().map(|k| {
            div()
                .min_w(px(18.))
                .h(px(18.))
                .px(theme::SPACE_1)
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .border_1()
                .border_color(theme::border())
                .bg(theme::surface())
                .text_size(theme::text_2xs())
                .font_weight(theme::WEIGHT_MEDIUM)
                .text_color(theme::text_muted())
                .child(k.to_string())
        }))
    }
}
