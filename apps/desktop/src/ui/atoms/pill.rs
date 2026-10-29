//! Rounded status pill tinted with a single color, with an optional leading icon.
//!
//! `Pill::new("#12", theme::success()).icon(IconName::GitPullRequest)`

use gpui::{div, prelude::*, App, Hsla, SharedString, Window};

use super::{Icon, IconName};
use crate::ui::theme;

#[derive(IntoElement)]
pub struct Pill {
    label: SharedString,
    color: Hsla,
    icon: Option<IconName>,
}

impl Pill {
    pub fn new(label: impl Into<SharedString>, color: Hsla) -> Self {
        Self { label: label.into(), color, icon: None }
    }

    /// Leading icon in the pill color.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl RenderOnce for Pill {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(theme::SPACE_1)
            .flex_shrink_0()
            .px(theme::SPACE_2)
            .py(theme::SPACE_0P5)
            .rounded(theme::RADIUS_SM)
            .text_size(theme::text_xs())
            .font_weight(theme::WEIGHT_MEDIUM)
            .bg(theme::tint_bg(self.color))
            .text_color(self.color)
            .children(self.icon.map(|n| Icon::new(n).size(theme::ICON_SM).color(self.color)))
            .child(self.label)
    }
}
