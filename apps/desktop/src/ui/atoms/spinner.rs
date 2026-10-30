//! Spinning loader icon for running work; a still icon under reduced motion.
//!
//! `Spinner::new("check-build").size(theme::ICON_MD).color(theme::warning())`

use gpui::{
    percentage, prelude::*, svg, Animation, AnimationExt, App, ElementId, Hsla, Pixels,
    SharedString, Transformation, Window,
};
use std::time::Duration;

use super::IconName;
use crate::ui::{motion, theme};

/// One turn; a loop, not an entrance, so it is not a motion token.
const TURN: Duration = Duration::from_millis(900);

#[derive(IntoElement)]
pub struct Spinner {
    id: SharedString,
    size: Pixels,
    color: Hsla,
}

impl Spinner {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            size: theme::ICON_MD,
            color: theme::text_muted(),
        }
    }

    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }
}

impl RenderOnce for Spinner {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let icon = svg()
            .path(IconName::Loader.path())
            .flex_shrink_0()
            .size(self.size)
            .text_color(self.color);
        if motion::reduced_motion() {
            return icon.into_any_element();
        }
        icon.with_animation(
            ElementId::Name(self.id),
            Animation::new(TURN).repeat(),
            |s, t| s.with_transformation(Transformation::rotate(percentage(t))),
        )
        .into_any_element()
    }
}
