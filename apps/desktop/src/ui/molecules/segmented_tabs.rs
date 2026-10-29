//! Segmented control: a row of mutually exclusive tabs.

use gpui::{div, prelude::*, App, ClickEvent, ElementId, SharedString, Window};
use std::rc::Rc;

use crate::ui::theme;

type SelectHandler = Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct SegmentedTabs {
    id: SharedString,
    items: Vec<(SharedString, bool)>,
    floating: bool,
    on_select: Option<SelectHandler>,
}

impl SegmentedTabs {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self { id: id.into(), items: vec![], floating: false, on_select: None }
    }

    /// Append a tab; `active` marks the selected one.
    pub fn tab(mut self, label: impl Into<SharedString>, active: bool) -> Self {
        self.items.push((label.into(), active));
        self
    }

    /// Pill-shaped variant on an overlay fill with a shadow, for bottom tab bars.
    pub fn floating(mut self) -> Self {
        self.floating = true;
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
            .items_center()
            .h(theme::CONTROL_H)
            .p(theme::SPACE_1)
            .gap(theme::SPACE_1)
            .rounded(theme::RADIUS_LG)
            .bg(theme::surface())
            .border_1()
            .border_color(theme::border())
            .when(self.floating, |d| d.h_auto().p(theme::SPACE_1).rounded(theme::RADIUS_FULL).bg(theme::overlay()).shadow_lg());
        let floating = self.floating;
        for (i, (label, active)) in self.items.into_iter().enumerate() {
            let on = self.on_select.clone();
            row = row.child(
                div()
                    .id(ElementId::Name(format!("{}-{i}", self.id).into()))
                    .flex()
                    .items_center()
                    .h_full()
                    .px(theme::SPACE_3)
                    .rounded(if floating { theme::RADIUS_FULL } else { theme::RADIUS_MD })
                    .when(floating, |d| d.h_auto().py(theme::SPACE_1))
                    .text_size(theme::text_sm())
                    .font_weight(theme::WEIGHT_MEDIUM)
                    .cursor_pointer()
                    .text_color(if active { theme::text() } else { theme::text_muted() })
                    .when(active, |d| d.bg(theme::surface_active()))
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
