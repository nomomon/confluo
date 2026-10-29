//! Clickable button with outline, ghost and link variants.
//!
//! `Button::new(id, label).variant(ButtonVariant::Ghost).size(Size::Sm).icon(IconName::Plus).on_click(..)`

use gpui::{div, prelude::*, App, ClickEvent, ElementId, SharedString, Window};

use super::{Icon, IconName};
use crate::ui::{theme, ClickHandler};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ButtonVariant {
    /// Bordered field-colored button (default).
    #[default]
    Outline,
    /// Borderless icon-style button that brightens on hover.
    Ghost,
    /// Accent-colored inline text link.
    Link,
}

/// Control size shared by buttons and other compact controls.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum Size {
    /// Compact (default): 12px text.
    #[default]
    Sm,
    /// Regular: 13px text, more padding.
    #[allow(dead_code)]
    Md,
}

#[derive(IntoElement)]
pub struct Button {
    id: SharedString,
    label: SharedString,
    icon: Option<IconName>,
    variant: ButtonVariant,
    size: Size,
    tooltip: Option<SharedString>,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self { id: id.into(), label: label.into(), icon: None, variant: ButtonVariant::default(), size: Size::default(), tooltip: None, on_click: None }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Leading glyph shown before the label.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Hover label, e.g. the action name and its shortcut.
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let base = div().id(ElementId::Name(self.id)).flex().items_center().gap(theme::SPACE_1).cursor_pointer();
        let styled = match self.variant {
            ButtonVariant::Outline => base
                .flex_shrink_0()
                .px(theme::SPACE_2P5)
                .py(theme::SPACE_1)
                .rounded(theme::RADIUS_MD)
                .border_1()
                .border_color(theme::border())
                .bg(theme::surface())
                .text_size(theme::text_xs())
                .hover(|s| s.bg(theme::surface_hover()))
                .active(|s| s.bg(theme::surface_active())),
            ButtonVariant::Ghost => {
                base.px(theme::SPACE_1P5).rounded(theme::RADIUS_MD).text_color(theme::text_muted()).hover(|s| s.bg(theme::surface_hover()).text_color(theme::text()))
                .active(|s| s.bg(theme::surface_active()))
            }
            ButtonVariant::Link => base.text_size(theme::text_xs()).text_color(theme::accent()).hover(|s| s.text_color(theme::text())),
        };
        styled
            .when(self.size == Size::Md, |el| el.px(theme::SPACE_3).py(theme::SPACE_1P5).text_size(theme::text_sm()))
            .children(self.icon.map(|n| Icon::new(n).color(if self.variant == ButtonVariant::Outline { theme::text() } else { theme::text_muted() })))
            .when(self.icon.is_some() && self.label.is_empty(), |el| el.py(theme::SPACE_1P5))
            .when(!self.label.is_empty(), |el| el.child(self.label))
            .when_some(self.tooltip, |el, t| el.tooltip(super::tooltip(t)))
            .when_some(self.on_click, |el, f| el.on_click(f))
    }
}
