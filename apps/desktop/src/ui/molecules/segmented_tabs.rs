//! Segmented control: a row of mutually exclusive tabs.

use gpui::{div, prelude::*, App, ClickEvent, ElementId, SharedString, Window};
use std::rc::Rc;

use crate::ui::theme;

type SelectHandler = Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct SegmentedTabs {
    id: SharedString,
    items: Vec<(SharedString, bool)>,
    on_select: Option<SelectHandler>,
}

impl SegmentedTabs {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self { id: id.into(), items: vec![], on_select: None }
    }

    /// Append a tab; `active` marks the selected one.
    pub fn tab(mut self, label: impl Into<SharedString>, active: bool) -> Self {
        self.items.push((label.into(), active));
        self
    }

    /// Called with the index of the clicked tab.
    pub fn on_select(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for SegmentedTabs {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let mut row = div()
            .flex()
            .flex_shrink_0()
            .p_0p5()
            .gap_0p5()
            .rounded_md()
            .bg(theme::field())
            .border_1()
            .border_color(theme::border());
        for (i, (label, active)) in self.items.into_iter().enumerate() {
            let on = self.on_select.clone();
            row = row.child(
                div()
                    .id(ElementId::Name(format!("{}-{i}", self.id).into()))
                    .px_2p5()
                    .py_0p5()
                    .rounded_sm()
                    .text_xs()
                    .cursor_pointer()
                    .text_color(if active { theme::text() } else { theme::muted() })
                    .when(active, |d| d.bg(theme::selected()))
                    .hover(|s| s.text_color(theme::text()))
                    .child(label)
                    .on_click(move |_: &ClickEvent, window, cx| {
                        if let Some(on) = &on {
                            on(&i, window, cx)
                        }
                    }),
            );
        }
        row
    }
}
