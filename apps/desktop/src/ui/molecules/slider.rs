//! Horizontal range like shadcn `Slider`: click or drag the track; values snap to `step`.
//!
//! `Slider::new("opacity", 0.7).range(0.2, 1.0).step(0.05).on_change(|v, w, cx| ..)`

use gpui::{
    canvas, div, prelude::*, px, App, Bounds, DragMoveEvent, ElementId, Empty, MouseButton,
    MouseDownEvent, Pixels, Point, SharedString, Window,
};
use std::cell::Cell;
use std::rc::Rc;

use crate::ui::{theme, ChangeHandler};

const TRACK_H: Pixels = px(4.);
const THUMB: Pixels = px(14.);

/// Drag payload; the preview view renders nothing.
#[derive(Clone)]
struct SliderDrag(SharedString);

impl Render for SliderDrag {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(IntoElement)]
pub struct Slider {
    id: SharedString,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    on_change: Option<ChangeHandler<f32>>,
}

impl Slider {
    pub fn new(id: impl Into<SharedString>, value: f32) -> Self {
        Self {
            id: id.into(),
            value,
            min: 0.,
            max: 1.,
            step: 0.01,
            on_change: None,
        }
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn on_change(mut self, f: impl Fn(&f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

/// Value at `x` within `bounds`, clamped and snapped to `step`.
pub fn value_at(x: Pixels, bounds: Bounds<Pixels>, min: f32, max: f32, step: f32) -> f32 {
    let w = f32::from(bounds.size.width).max(1.);
    let t = (f32::from(x - bounds.origin.x) / w).clamp(0., 1.);
    let v = min + t * (max - min);
    let v = if step > 0. {
        min + ((v - min) / step).round() * step
    } else {
        v
    };
    v.clamp(min, max)
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let bounds = window
            .use_keyed_state(
                ElementId::Name(format!("{}-bounds", self.id).into()),
                cx,
                |_, _| Rc::new(Cell::new(Bounds::<Pixels>::default())),
            )
            .read(cx)
            .clone();
        let t = ((self.value - self.min) / (self.max - self.min).max(f32::EPSILON)).clamp(0., 1.);
        let (min, max, step) = (self.min, self.max, self.step);
        let emit = move |f: &Option<ChangeHandler<f32>>,
                         pos: Point<Pixels>,
                         b: Bounds<Pixels>,
                         w: &mut Window,
                         cx: &mut App| {
            if let Some(f) = f {
                f(&value_at(pos.x, b, min, max, step), w, cx)
            }
        };
        let (f_down, f_drag, b_down, b_canvas) = (
            self.on_change.clone(),
            self.on_change.clone(),
            bounds.clone(),
            bounds.clone(),
        );
        let (id, own) = (self.id.clone(), self.id.clone());
        let track = div()
            .relative()
            .w_full()
            .h(TRACK_H)
            .rounded(theme::RADIUS_FULL)
            .bg(theme::surface_active())
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(gpui::relative(t))
                    .rounded(theme::RADIUS_FULL)
                    .bg(theme::text()),
            )
            .child(
                canvas(move |b, _, _| b_canvas.set(b), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            );
        let thumb = div()
            .absolute()
            .top(px(1.))
            .left(gpui::relative(t))
            .ml(-THUMB / 2.)
            .size(THUMB)
            .rounded(theme::RADIUS_FULL)
            .border_2()
            .border_color(theme::text())
            .bg(theme::bg())
            .shadow_sm();
        div()
            .id(ElementId::Name(self.id))
            .relative()
            .flex()
            .items_center()
            .min_w(px(140.))
            .h(px(16.))
            .cursor_pointer()
            .child(track)
            .child(thumb)
            .on_mouse_down(MouseButton::Left, move |ev: &MouseDownEvent, w, cx| {
                emit(&f_down, ev.position, b_down.get(), w, cx)
            })
            .on_drag(SliderDrag(id), |d, _, _, cx| cx.new(|_| d.clone()))
            .on_drag_move(move |ev: &DragMoveEvent<SliderDrag>, w, cx| {
                if ev.drag(cx).0 == own {
                    emit(&f_drag, ev.event.position, bounds.get(), w, cx)
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, size};

    #[test]
    fn snaps_and_clamps() {
        let b = Bounds {
            origin: point(px(10.), px(0.)),
            size: size(px(100.), px(4.)),
        };
        assert_eq!(value_at(px(10.), b, 0., 1., 0.1), 0.);
        assert!((value_at(px(64.), b, 0., 1., 0.1) - 0.5).abs() < 1e-5);
        assert_eq!(value_at(px(500.), b, 0., 1., 0.1), 1.);
    }
}
