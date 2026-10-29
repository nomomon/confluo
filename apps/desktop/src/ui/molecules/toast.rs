//! Small floating message pinned to the bottom center of its (relative) parent.

use gpui::{div, prelude::*, App, SharedString, Window};

use crate::ui::{motion, theme};

#[derive(IntoElement)]
pub struct Toast(SharedString);

impl Toast {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self(text.into())
    }
}

impl RenderOnce for Toast {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let pill = div()
            .px(theme::SPACE_3)
            .py(theme::SPACE_1P5)
            .rounded(theme::RADIUS_FULL)
            .border_1()
            .border_color(theme::BORDER)
            .bg(theme::OVERLAY)
            .shadow_lg()
            .text_size(theme::text_xs())
            .child(self.0);
        div().absolute().bottom(theme::SPACE_6).left_0().right_0().flex().justify_center().child(motion::fade_in(pill, "toast"))
    }
}
