//! Text helpers: section headings and inline error text.

use gpui::{div, prelude::*, App, SharedString, Window};

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
        div().px(theme::SPACE_2).pt(theme::SPACE_3).pb(theme::SPACE_1).text_size(theme::text_xs()).font_weight(theme::WEIGHT_MEDIUM).text_color(theme::text_subtle()).child(self.0)
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
        div().text_size(theme::text_xs()).text_color(theme::danger()).child(self.0)
    }
}
