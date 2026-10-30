//! Rows for `DropdownMenu` and `ContextMenu`, like shadcn's menu parts.
//!
//! `MenuItem::new("rename", "Rename").icon(IconName::Pencil).shortcut("⌘ R").on_click(..)`,
//! `MenuItem::new("delete", "Delete").destructive()`, `MenuSeparator`, `MenuLabel::new("Project")`.

use gpui::{div, prelude::*, px, App, ClickEvent, ElementId, SharedString, Window};

use crate::ui::atoms::{Icon, IconName, Kbd, Separator};
use crate::ui::{theme, ClickHandler};

#[derive(IntoElement)]
pub struct MenuItem {
    id: SharedString,
    label: SharedString,
    icon: Option<IconName>,
    shortcut: Option<SharedString>,
    destructive: bool,
    disabled: bool,
    tooltip: Option<SharedString>,
    on_click: Option<ClickHandler>,
}

impl MenuItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self { id: id.into(), label: label.into(), icon: None, shortcut: None, destructive: false, disabled: false, tooltip: None, on_click: None }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn shortcut(mut self, keys: impl Into<SharedString>) -> Self {
        self.shortcut = Some(keys.into());
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    /// Greyed out and not clickable; pair with `tooltip` to say why.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl RenderOnce for MenuItem {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let color = if self.destructive { theme::danger() } else { theme::text() };
        div()
            .id(ElementId::Name(self.id))
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .px(theme::SPACE_2)
            .py(theme::SPACE_1P5)
            .rounded(theme::RADIUS_SM)
            .text_color(if self.disabled { theme::text_subtle() } else { color })
            .when(!self.disabled, |d| d.cursor_pointer().hover(|s| s.bg(if self.destructive { theme::tint_bg(theme::danger()) } else { theme::surface_hover() })))
            .children(self.icon.map(|n| Icon::new(n).color(if self.destructive { theme::danger() } else { theme::text_muted() })))
            .child(div().flex_1().child(self.label))
            .children(self.shortcut.map(Kbd::new))
            .when_some(self.tooltip, |d, t| d.tooltip(crate::ui::atoms::tooltip(t)))
            .when_some(self.on_click.filter(|_| !self.disabled), |d, f| d.on_click(f))
    }
}

/// Hairline between groups of menu items.
#[derive(IntoElement)]
pub struct MenuSeparator;

impl RenderOnce for MenuSeparator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().my(theme::SPACE_1).mx(px(-4.)).child(Separator::new())
    }
}

/// Small muted heading inside a menu.
#[derive(IntoElement)]
pub struct MenuLabel(SharedString);

impl MenuLabel {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self(text.into())
    }
}

impl RenderOnce for MenuLabel {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().px(theme::SPACE_2).py(theme::SPACE_1).text_size(theme::text_xs()).font_weight(theme::WEIGHT_MEDIUM).text_color(theme::text_subtle()).child(self.0)
    }
}
