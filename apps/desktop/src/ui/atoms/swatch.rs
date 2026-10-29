//! Round color swatch with an active ring.

use gpui::{div, prelude::*, px, App, ClickEvent, ElementId, Hsla, SharedString, Window};

use crate::ui::{theme, ClickHandler};

#[derive(IntoElement)]
pub struct Swatch {
    id: SharedString,
    color: Hsla,
    active: bool,
    on_click: Option<ClickHandler>,
}

impl Swatch {
    pub fn new(id: impl Into<SharedString>, color: Hsla) -> Self {
        Self { id: id.into(), color, active: false, on_click: None }
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

impl RenderOnce for Swatch {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id(ElementId::Name(self.id))
            .size(px(22.))
            .rounded_full()
            .border_2()
            .border_color(if self.active { theme::accent() } else { theme::border() })
            .bg(self.color)
            .cursor_pointer()
            .when_some(self.on_click, |el, f| el.on_click(f))
    }
}
