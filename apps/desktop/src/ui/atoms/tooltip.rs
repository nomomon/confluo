//! Hover tooltip: a small overlay label. Use through `Button::tooltip` or `tooltip(text)`.

use gpui::{div, prelude::*, AnyView, App, Context, SharedString, Window};

use crate::ui::theme;

pub struct Tooltip(SharedString);

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(theme::SPACE_2)
            .py(theme::SPACE_1)
            .rounded(theme::RADIUS_SM)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
            .font_family(theme::ui_font())
            .child(self.0.clone())
    }
}

/// Builder for GPUI's `.tooltip(..)` on stateful elements.
pub fn tooltip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| Tooltip(text.clone())).into()
}
