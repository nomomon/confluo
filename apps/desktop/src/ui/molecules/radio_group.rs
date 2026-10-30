//! Vertical set of mutually exclusive options like shadcn `RadioGroup`.
//!
//! `RadioGroup::new("open-in").option("App").option("Browser").selected(0).on_change(..)`

use gpui::{div, prelude::*, px, App, ClickEvent, ElementId, SharedString, Window};
use std::rc::Rc;

use crate::ui::{theme, ChangeHandler};

#[derive(IntoElement)]
pub struct RadioGroup {
    id: SharedString,
    options: Vec<SharedString>,
    selected: usize,
    on_change: Option<ChangeHandler<usize>>,
}

impl RadioGroup {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self { id: id.into(), options: vec![], selected: 0, on_change: None }
    }

    pub fn option(mut self, label: impl Into<SharedString>) -> Self {
        self.options.push(label.into());
        self
    }

    pub fn selected(mut self, i: usize) -> Self {
        self.selected = i;
        self
    }

    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for RadioGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().flex().flex_col().gap(theme::SPACE_2).children(self.options.into_iter().enumerate().map(|(i, label)| {
            let on = i == self.selected;
            let dot = div()
                .size(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::RADIUS_FULL)
                .border_1()
                .border_color(if on { theme::text() } else { theme::border() })
                .when(on, |d| d.child(div().size(px(8.)).rounded(theme::RADIUS_FULL).bg(theme::text())));
            let f = self.on_change.clone();
            div()
                .id(ElementId::Name(format!("{}-{i}", self.id).into()))
                .flex()
                .items_center()
                .gap(theme::SPACE_2)
                .cursor_pointer()
                .child(dot)
                .child(label)
                .when_some(f, |d, f| d.on_click(move |_: &ClickEvent, w, cx| f(&i, w, cx)))
        }))
    }
}
