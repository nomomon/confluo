//! Round author avatar: the loaded image, else a subtle initial.

use gpui::{div, img, prelude::*, App, Image, SharedString, Window};
use std::sync::Arc;

use crate::ui::theme;

#[derive(IntoElement)]
pub struct Avatar {
    name: SharedString,
    image: Option<Arc<Image>>,
    size: gpui::Pixels,
}

impl Avatar {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self { name: name.into(), image: None, size: theme::SPACE_4 }
    }

    /// Diameter; 16px by default, 24px in the activity timeline.
    pub fn size(mut self, size: gpui::Pixels) -> Self {
        self.size = size;
        self
    }

    pub fn image(mut self, image: Option<Arc<Image>>) -> Self {
        self.image = image;
        self
    }
}

impl RenderOnce for Avatar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let base = div().flex().flex_shrink_0().items_center().justify_center().size(self.size).rounded(theme::RADIUS_FULL).overflow_hidden();
        match self.image {
            Some(image) => base.child(img(image).size_full().rounded(theme::RADIUS_FULL)),
            None => {
                let initial: String = self.name.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
                base.bg(theme::surface()).text_color(theme::text_subtle()).text_size(if self.size > theme::SPACE_5 { theme::text_xs() } else { theme::text_2xs() }).child(initial)
            }
        }
    }
}
