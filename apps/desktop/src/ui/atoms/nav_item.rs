//! Selectable navigation row (sidebar entries, menu items, list pickers).

use gpui::{div, prelude::*, AnyElement, App, ClickEvent, Div, ElementId, MouseButton, MouseDownEvent, SharedString, StyleRefinement, Window};

use crate::ui::{theme, ClickHandler};

#[derive(IntoElement)]
pub struct NavItem {
    base: Div,
    id: SharedString,
    label: SharedString,
    active: bool,
    trailing: Option<AnyElement>,
    on_click: Option<ClickHandler>,
    on_right_click: Option<Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>>,
}

impl NavItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self { base: div(), id: id.into(), label: label.into(), active: false, trailing: None, on_click: None, on_right_click: None }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
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
        let group = self.id.clone();
        self.base
            .id(ElementId::Name(self.id))
            .group(group.clone())
            .flex()
            .items_center()
            .gap(theme::SPACE_1)
            .px(theme::SPACE_2)
            .py(theme::SPACE_1)
            .rounded(theme::RADIUS_MD)
            .text_size(theme::TEXT_SM)
            .cursor_pointer()
            .text_color(if active { theme::TEXT } else { theme::TEXT_MUTED })
            .when(active, |d| d.bg(theme::SURFACE_ACTIVE))
            .hover(|s| s.bg(theme::SURFACE_HOVER))
            .active(|s| s.bg(theme::SURFACE_ACTIVE))
            .child(div().flex_1().min_w(theme::SPACE_0).truncate().child(self.label))
            .when_some(self.trailing, |el, t| el.child(div().flex_shrink_0().opacity(0.).group_hover(group, |s| s.opacity(1.)).child(t)))
            .when_some(self.on_click, |el, f| el.on_click(f))
            .when_some(self.on_right_click, |el, f| el.on_mouse_down(MouseButton::Right, f))
    }
}
