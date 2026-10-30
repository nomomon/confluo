//! Loading placeholder block that gently pulses (static under reduced motion).
//!
//! `Skeleton::new().w(px(120.)).h(px(12.))`

use gpui::{
    div, prelude::*, pulsating_between, Animation, AnimationExt, App, Div, StyleRefinement, Window,
};
use std::time::Duration;

use crate::ui::{motion, theme};

#[derive(IntoElement)]
pub struct Skeleton {
    base: Div,
}

impl Skeleton {
    pub fn new() -> Self {
        Self { base: div() }
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Skeleton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let el = self
            .base
            .rounded(theme::RADIUS_MD)
            .bg(theme::surface_active());
        if motion::reduced_motion() {
            return el.into_any_element();
        }
        el.with_animation(
            "skeleton",
            Animation::new(Duration::from_millis(1400))
                .repeat()
                .with_easing(pulsating_between(0.5, 1.)),
            |el, t| el.opacity(t),
        )
        .into_any_element()
    }
}
