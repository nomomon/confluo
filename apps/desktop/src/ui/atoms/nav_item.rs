//! Selectable navigation row (sidebar entries, menu items, list pickers).

use gpui::{div, prelude::*, App, ClickEvent, Div, ElementId, SharedString, StyleRefinement, Window};

use crate::ui::{theme, ClickHandler};

#[derive(IntoElement)]
pub struct NavItem {
    base: Div,
    id: SharedString,
    label: SharedString,
    active: bool,
    on_click: Option<ClickHandler>,
}

impl NavItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self { base: div(), id: id.into(), label: label.into(), active: false, on_click: None }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
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
        self.base
            .id(ElementId::Name(self.id))
            .px(theme::SPACE_2)
            .py(theme::SPACE_1)
            .rounded(theme::RADIUS_MD)
            .text_size(theme::TEXT_SM)
            .truncate()
            .cursor_pointer()
            .text_color(if active { theme::TEXT } else { theme::TEXT_MUTED })
            .when(active, |d| d.bg(theme::SURFACE_ACTIVE))
            .hover(|s| s.bg(theme::SURFACE_HOVER))
            .active(|s| s.bg(theme::SURFACE_ACTIVE))
            .child(self.label)
            .when_some(self.on_click, |el, f| el.on_click(f))
    }
}
