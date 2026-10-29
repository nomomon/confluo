//! Warning banner for non-fatal status messages.

use gpui::{div, prelude::*, App, SharedString, Window};

use crate::ui::{motion, theme};

#[derive(IntoElement)]
pub struct Notice(SharedString);

impl Notice {
    pub fn warning(text: impl Into<SharedString>) -> Self {
        Self(text.into())
    }
}

impl RenderOnce for Notice {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let el = div()
            .text_size(theme::TEXT_XS)
            .text_color(theme::WARNING)
            .px(theme::SPACE_2)
            .py(theme::SPACE_1)
            .rounded(theme::RADIUS_MD)
            .bg(theme::tint_bg(theme::WARNING))
            .line_clamp(3)
            .child(self.0);
        motion::fade_in(el, "notice")
    }
}
