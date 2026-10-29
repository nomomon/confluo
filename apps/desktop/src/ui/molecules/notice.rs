//! Warning banner for non-fatal status messages.

use gpui::{div, prelude::*, App, SharedString, Window};

use crate::ui::theme;

#[derive(IntoElement)]
pub struct Notice(SharedString);

impl Notice {
    pub fn warning(text: impl Into<SharedString>) -> Self {
        Self(text.into())
    }
}

impl RenderOnce for Notice {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .text_xs()
            .text_color(theme::yellow())
            .px_2()
            .py_1()
            .rounded_md()
            .bg(theme::yellow().opacity(0.08))
            .line_clamp(3)
            .child(self.0)
    }
}
