//! Small rounded label like shadcn `Badge`, with variants.
//!
//! `Badge::new("Draft").variant(BadgeVariant::Outline)`, `Badge::new("3").tint(theme::success())`

use gpui::{div, prelude::*, App, Hsla, SharedString, Window};

use crate::ui::theme;

#[allow(dead_code)] // kit API: every shadcn variant
#[derive(Clone, Copy, PartialEq, Default)]
pub enum BadgeVariant {
    /// High contrast fill.
    Default,
    /// Subtle surface fill (default).
    #[default]
    Secondary,
    Outline,
    Destructive,
    /// A semantic color's tinted fill.
    Tint(Hsla),
}

#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    variant: BadgeVariant,
}

impl Badge {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self { label: label.into(), variant: BadgeVariant::default() }
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn tint(self, color: Hsla) -> Self {
        self.variant(BadgeVariant::Tint(color))
    }
}

impl RenderOnce for Badge {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (bg, fg, border) = match self.variant {
            BadgeVariant::Default => (theme::text(), theme::bg(), theme::text()),
            BadgeVariant::Secondary => (theme::surface_active(), theme::text_muted(), theme::border_subtle()),
            BadgeVariant::Outline => (gpui::transparent_black(), theme::text_muted(), theme::border()),
            BadgeVariant::Destructive => (theme::tint_bg(theme::danger()), theme::danger(), theme::tint_bg(theme::danger())),
            BadgeVariant::Tint(c) => (theme::tint_bg(c), c, theme::tint_bg(c)),
        };
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .px(theme::SPACE_2)
            .py(theme::SPACE_0P5)
            .rounded(theme::RADIUS_FULL)
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_color(fg)
            .text_size(theme::text_2xs())
            .font_weight(theme::WEIGHT_MEDIUM)
            .child(self.label)
    }
}
