//! Selectable navigation row (sidebar entries, menu items, list pickers).

use gpui::{div, prelude::*, AnyElement, App, ClickEvent, Div, ElementId, MouseButton, MouseDownEvent, SharedString, StyleRefinement, Window};

use super::{Icon, IconName, ListItem};
use crate::ui::{theme, ClickHandler, MouseDownHandler};

#[derive(IntoElement)]
pub struct NavItem {
    base: Div,
    id: SharedString,
    label: SharedString,
    active: bool,
    compact: bool,
    icon: Option<IconName>,
    trailing: Option<AnyElement>,
    on_click: Option<ClickHandler>,
    on_right_click: Option<MouseDownHandler>,
}

impl NavItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self { base: div(), id: id.into(), label: label.into(), active: false, compact: false, icon: None, trailing: None, on_click: None, on_right_click: None }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Icon only, centered, with the label as a tooltip (collapsed sidebar rail).
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Element on the right edge, shown only while the row is hovered.
    pub fn trailing_on_hover(mut self, el: impl IntoElement) -> Self {
        self.trailing = Some(el.into_any_element());
        self
    }

    pub fn on_right_click(mut self, f: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_right_click = Some(Box::new(f));
        self
    }

    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl Styled for NavItem {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for NavItem {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let active = self.active;
        let compact = self.compact && self.icon.is_some();
        let group = self.id.clone();
        let label = self.label.clone();
        self.base
            .id(ElementId::Name(self.id))
            .group(group.clone())
            .flex()
            .items_center()
            .gap(theme::SPACE_2P5)
            .px(theme::SPACE_2P5)
            .py(theme::item_pad_y())
            .list_item(active)
            .text_size(theme::text_sm())
            .text_color(if active { theme::text() } else { theme::text_muted() })
            .children(self.icon.map(|n| Icon::new(n).color(if active { theme::text() } else { theme::text_subtle() })))
            .when(compact, |d| d.justify_center().px(theme::SPACE_2).tooltip(super::tooltip(label)))
            .when(!compact, |d| d.child(div().flex_1().min_w(theme::SPACE_0).truncate().child(self.label)))
            .when_some(self.trailing.filter(|_| !compact), |el, t| el.child(div().flex_shrink_0().opacity(0.).group_hover(group, |s| s.opacity(1.)).child(t)))
            .when_some(self.on_click, |el, f| el.on_click(f))
            .when_some(self.on_right_click, |el, f| el.on_mouse_down(MouseButton::Right, f))
    }
}
