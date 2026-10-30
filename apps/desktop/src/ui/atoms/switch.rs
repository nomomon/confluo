//! On/off toggle like shadcn `Switch`: a pill track with a sliding knob.
//!
//! `Switch::new("hide-bots", on).on_change(|on, window, cx| ..)`

use gpui::{div, prelude::*, px, App, ClickEvent, ElementId, Pixels, SharedString, Window};
use std::rc::Rc;

use crate::ui::{theme, ChangeHandler};

const TRACK_W: Pixels = px(32.);
const TRACK_H: Pixels = px(18.);
const KNOB: Pixels = px(14.);

#[derive(IntoElement)]
pub struct Switch {
    id: SharedString,
    checked: bool,
    disabled: bool,
    on_change: Option<ChangeHandler<bool>>,
}

impl Switch {
    pub fn new(id: impl Into<SharedString>, checked: bool) -> Self {
        Self { id: id.into(), checked, disabled: false, on_change: None }
    }

    #[allow(dead_code)] // kit API
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the new value.
    pub fn on_change(mut self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let on = self.checked;
        let knob = div().size(KNOB).rounded(theme::RADIUS_FULL).bg(if on { theme::bg() } else { theme::text_muted() }).shadow_sm();
        div()
            .id(ElementId::Name(self.id))
            .flex()
            .flex_shrink_0()
            .items_center()
            .w(TRACK_W)
            .h(TRACK_H)
            .px(px(2.))
            .rounded(theme::RADIUS_FULL)
            .border_1()
            .border_color(if on { theme::text() } else { theme::border() })
            .bg(if on { theme::text() } else { theme::surface_active() })
            .when(on, |d| d.justify_end())
            .when(self.disabled, |d| d.opacity(0.5))
            .when(!self.disabled, |d| d.cursor_pointer())
            .child(knob)
            .when_some(self.on_change.filter(|_| !self.disabled), |d, f| d.on_click(move |_: &ClickEvent, w, cx| f(&!on, w, cx)))
    }
}
