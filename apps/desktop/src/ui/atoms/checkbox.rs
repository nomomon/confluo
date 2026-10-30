//! Square check box with an optional trailing label, like shadcn `Checkbox`.
//!
//! `Checkbox::new("task-1", done).label("Ship it").on_change(..)`

use gpui::{div, prelude::*, px, App, ClickEvent, ElementId, Pixels, SharedString, Window};
use std::rc::Rc;

use super::{Icon, IconName};
use crate::ui::{theme, ChangeHandler};

const BOX: Pixels = px(16.);

#[derive(IntoElement)]
pub struct Checkbox {
    id: SharedString,
    checked: bool,
    label: Option<SharedString>,
    on_change: Option<ChangeHandler<bool>>,
}

impl Checkbox {
    pub fn new(id: impl Into<SharedString>, checked: bool) -> Self {
        Self {
            id: id.into(),
            checked,
            label: None,
            on_change: None,
        }
    }

    #[allow(dead_code)] // kit API
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[allow(dead_code)] // kit API
    pub fn on_change(mut self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let on = self.checked;
        let mark = div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(BOX)
            .rounded(px(4.))
            .border_1()
            .border_color(if on { theme::text() } else { theme::border() })
            .when(on, |d| {
                d.bg(theme::text()).child(
                    Icon::new(IconName::Check)
                        .size(theme::ICON_SM)
                        .color(theme::bg()),
                )
            });
        div()
            .id(ElementId::Name(self.id))
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .child(mark)
            .children(self.label)
            .when_some(self.on_change, |d, f| {
                d.cursor_pointer()
                    .on_click(move |_: &ClickEvent, w, cx| f(&!on, w, cx))
            })
    }
}
