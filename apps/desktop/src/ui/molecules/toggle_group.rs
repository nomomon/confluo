//! Compact single-choice button group like shadcn `ToggleGroup` (type="single").
//!
//! `ToggleGroup::new("theme").item("System", None, true).item("Dark", Some(IconName::..), false).on_change(..)`

use gpui::{div, prelude::*, px, App, ClickEvent, ElementId, SharedString, Window};
use std::rc::Rc;

use crate::ui::atoms::{Icon, IconName};
use crate::ui::{theme, ChangeHandler};

#[derive(IntoElement)]
pub struct ToggleGroup {
    id: SharedString,
    items: Vec<(SharedString, Option<IconName>, bool)>,
    on_change: Option<ChangeHandler<usize>>,
}

impl ToggleGroup {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            items: vec![],
            on_change: None,
        }
    }

    pub fn item(
        mut self,
        label: impl Into<SharedString>,
        icon: Option<IconName>,
        active: bool,
    ) -> Self {
        self.items.push((label.into(), icon, active));
        self
    }

    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ToggleGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_shrink_0()
            .p(px(2.))
            .gap(px(2.))
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::border())
            .bg(theme::surface())
            .children(
                self.items
                    .into_iter()
                    .enumerate()
                    .map(|(i, (label, icon, active))| {
                        let f = self.on_change.clone();
                        div()
                            .id(ElementId::Name(format!("{}-{i}", self.id).into()))
                            .flex()
                            .items_center()
                            .gap(theme::SPACE_1)
                            .h(px(24.))
                            .px(theme::SPACE_2P5)
                            .rounded(theme::RADIUS_SM)
                            .text_size(theme::text_xs())
                            .font_weight(theme::WEIGHT_MEDIUM)
                            .cursor_pointer()
                            .text_color(if active {
                                theme::text()
                            } else {
                                theme::text_muted()
                            })
                            .when(active, |d| d.bg(theme::surface_active()).shadow_sm())
                            .when(!active, |d| d.hover(|s| s.text_color(theme::text())))
                            .children(icon.map(|n| {
                                Icon::new(n).size(theme::ICON_SM).color(if active {
                                    theme::text()
                                } else {
                                    theme::text_muted()
                                })
                            }))
                            .child(label)
                            .when_some(f, |d, f| {
                                d.on_click(move |_: &ClickEvent, w, cx| f(&i, w, cx))
                            })
                    }),
            )
    }
}
