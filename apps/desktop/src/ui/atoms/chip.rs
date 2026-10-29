//! Toggleable filter chip.

use gpui::{div, prelude::*, App, ClickEvent, ElementId, SharedString, Window};

use crate::ui::{theme, ClickHandler};

#[derive(IntoElement)]
pub struct Chip {
    id: SharedString,
    label: SharedString,
    active: bool,
    on_click: Option<ClickHandler>,
}

impl Chip {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self { id: id.into(), label: label.into(), active: false, on_click: None }
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

impl RenderOnce for Chip {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let active = self.active;
        div()
            .id(ElementId::Name(self.id))
            .px_2p5()
            .py_0p5()
            .rounded_full()
            .border_1()
            .text_xs()
            .cursor_pointer()
            .border_color(if active { theme::accent().opacity(0.6) } else { theme::border() })
            .bg(if active { theme::accent().opacity(0.15) } else { theme::field() })
            .text_color(if active { theme::text() } else { theme::muted() })
            .hover(|s| s.bg(theme::hover()))
            .child(self.label)
            .when_some(self.on_click, |el, f| el.on_click(f))
    }
}
