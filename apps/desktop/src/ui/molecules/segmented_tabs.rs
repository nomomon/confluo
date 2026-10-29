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
            .p(theme::SPACE_0P5)
            .gap(theme::SPACE_0P5)
            .rounded(theme::RADIUS_MD)
            .bg(theme::SURFACE)
            .border_1()
            .border_color(theme::BORDER)
            .when(self.floating, |d| d.p(theme::SPACE_1).rounded(theme::RADIUS_FULL).bg(theme::OVERLAY).shadow_lg());
        let floating = self.floating;
        for (i, (label, active)) in self.items.into_iter().enumerate() {
            let on = self.on_select.clone();
            row = row.child(
                div()
                    .id(ElementId::Name(format!("{}-{i}", self.id).into()))
                    .px(theme::SPACE_2P5)
                    .py(theme::SPACE_0P5)
                    .rounded(if floating { theme::RADIUS_FULL } else { theme::RADIUS_SM })
                    .when(floating, |d| d.px(theme::SPACE_3).py(theme::SPACE_1))
                    .text_size(theme::text_xs())
                    .cursor_pointer()
                    .text_color(if active { theme::TEXT } else { theme::TEXT_MUTED })
                    .when(active, |d| d.bg(theme::SURFACE_ACTIVE))
                    .hover(|s| s.text_color(theme::TEXT))
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
