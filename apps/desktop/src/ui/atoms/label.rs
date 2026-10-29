//! Text helpers: section headings and inline error text.

use gpui::{div, prelude::*, App, FontWeight, SharedString, Window};

use crate::ui::theme;

/// Small faint heading used to group navigation items.
#[derive(IntoElement)]
pub struct SectionLabel(SharedString);

impl SectionLabel {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self(text.into())
    }
}

impl RenderOnce for SectionLabel {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().px_2().pt_3().pb_1().text_xs().font_weight(FontWeight::MEDIUM).text_color(theme::faint()).child(self.0)
    }
}

/// Small red error message.
#[derive(IntoElement)]
pub struct ErrorText(SharedString);

impl ErrorText {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self(text.into())
    }
}

impl RenderOnce for ErrorText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().text_xs().text_color(theme::red()).child(self.0)
    }
}
