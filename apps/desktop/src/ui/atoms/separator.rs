//! Hairline divider, horizontal by default: `Separator::new()` or `Separator::vertical()`.

use gpui::{div, prelude::*, px, App, Window};

use crate::ui::theme;

#[derive(IntoElement, Default)]
pub struct Separator {
    vertical: bool,
}

impl Separator {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)] // kit API
    pub fn vertical() -> Self {
        Self { vertical: true }
    }
}

impl RenderOnce for Separator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let d = div().flex_shrink_0().bg(theme::border_subtle());
        if self.vertical {
            d.w(px(1.)).h_full()
        } else {
            d.h(px(1.)).w_full()
        }
    }
}
