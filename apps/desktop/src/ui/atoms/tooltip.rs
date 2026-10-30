//! Hover tooltip: a small overlay label. Use through `Button::tooltip` or `tooltip(text)`.
//! Text after a double space renders as a `Kbd` shortcut: `tooltip("Refresh  ⌘ R")`.

use gpui::{div, prelude::*, AnyView, App, Context, SharedString, Window};

use super::Kbd;
use crate::ui::theme;

pub struct Tooltip(SharedString);

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (label, keys) = match self.0.split_once("  ") {
            Some((l, k)) => (l.to_string(), Some(k.trim().to_string())),
            None => (self.0.to_string(), None),
        };
        div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .px(theme::SPACE_2)
            .py(theme::SPACE_1)
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_md()
            .text_size(theme::text_xs())
            .text_color(theme::text())
            .font_family(theme::ui_font())
            .child(label)
            .children(keys.map(Kbd::new))
    }
}

/// Builder for GPUI's `.tooltip(..)` on stateful elements.
pub fn tooltip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |window, cx| {
        // An open menu or popover elsewhere owns the screen: show nothing (an empty view).
        if crate::ui::layer::blocks(window.mouse_position()) {
            return cx.new(|_| Empty).into();
        }
        cx.new(|_| Tooltip(text.clone())).into()
    }
}

/// What a suppressed tooltip or hover card shows.
pub struct Empty;

impl Render for Empty {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
