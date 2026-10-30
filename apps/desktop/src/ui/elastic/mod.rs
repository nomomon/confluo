//! gpui glue for `motion::interactive::ElasticScroll` (rubber-band overscroll, one per scroller key):
//! scroller adapters, per-key state and frame requests. The scroll offset itself never leaves its range: the stretch
//! is a visual translate of the content (`shift`), so lists keep their scroll-to-item and spacer logic.
//! Mouse wheels (line deltas) and reduced motion hard-stop at the edges.

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

use gpui::{
    point, px, Div, InteractiveElement, ListState, Pixels, ScrollDelta, ScrollHandle,
    ScrollWheelEvent, Stateful, Styled, Window,
};

use super::scroll::Axis;

pub use super::motion::interactive::{ElasticScroll, Pin};

/// What a scroller reports for one axis: offset (GPUI sign, 0 at the start, `-max` at the end), max and viewport length.
pub trait Scroller {
    fn pos(&self, axis: Axis) -> (f32, f32, f32);
    fn pin(&self, axis: Axis, at: f32);
}

impl Scroller for ScrollHandle {
    fn pos(&self, axis: Axis) -> (f32, f32, f32) {
        let (o, m, b) = (self.offset(), self.max_offset(), self.bounds().size);
        match axis {
            Axis::Y => (o.y.into(), m.height.into(), b.height.into()),
            Axis::X => (o.x.into(), m.width.into(), b.width.into()),
        }
    }
    fn pin(&self, axis: Axis, at: f32) {
        let o = self.offset();
        self.set_offset(match axis {
            Axis::Y => point(o.x, px(at)),
            Axis::X => point(px(at), o.y),
        });
    }
}

impl Scroller for ListState {
    fn pos(&self, _: Axis) -> (f32, f32, f32) {
        (
            self.scroll_px_offset_for_scrollbar().y.into(),
            self.max_offset_for_scrollbar().height.into(),
            self.viewport_bounds().size.height.into(),
        )
    }
    fn pin(&self, _: Axis, at: f32) {
        self.set_offset_from_scrollbar(point(px(0.), px(at)));
    }
}

thread_local! {
    static STRETCH: RefCell<HashMap<String, ElasticScroll>> = Default::default();
    static HANDLES: RefCell<HashMap<&'static str, ScrollHandle>> = Default::default();
}

/// A scroll handle kept per pane `key`, for panes that had none.
pub fn handle(key: &'static str) -> ScrollHandle {
    HANDLES.with(|m| m.borrow_mut().entry(key).or_default().clone())
}

/// Handle one wheel event for the scroller `key` (call after the scroller moved, e.g. from its own listener).
/// Returns whether to redraw.
pub fn wheel(key: &str, axis: Axis, ev: &ScrollWheelEvent, s: &dyn Scroller) -> bool {
    let (offset, max, extent) = s.pos(axis);
    let (changed, pin) = feed(key, axis, ev, offset, max, extent);
    match pin {
        Some(Pin::Start) => s.pin(axis, 0.),
        Some(Pin::End) => s.pin(axis, -max),
        None => {}
    }
    changed
}

/// `wheel` for a scroller kept in app state: given its offset (GPUI sign), max and viewport length, returns
/// whether to redraw and where to hold it.
pub fn feed(
    key: &str,
    axis: Axis,
    ev: &ScrollWheelEvent,
    offset: f32,
    max: f32,
    extent: f32,
) -> (bool, Option<Pin>) {
    let (precise, delta) = match ev.delta {
        ScrollDelta::Pixels(p) => (true, p),
        ScrollDelta::Lines(l) => (false, point(px(l.x), px(l.y))),
    };
    STRETCH.with(|m| {
        m.borrow_mut().entry(key.to_string()).or_default().feed(
            axis,
            ev.touch_phase,
            precise,
            super::motion::reduced_motion(),
            delta.x.into(),
            delta.y.into(),
            offset >= -0.5,
            offset <= -max + 0.5,
            extent,
            Instant::now(),
        )
    })
}

/// Current stretch of `key` as a translate (steps its spring).
pub fn shift(key: &str) -> Pixels {
    px(STRETCH.with(|m| {
        m.borrow_mut()
            .get_mut(key)
            .map(|s| s.shown(Instant::now()).0)
            .unwrap_or(0.)
    }))
}

/// Call at the start of the window's render: keeps frames coming while any stretch springs back
/// (a pane no longer drawn drops its spring, so this never spins).
pub fn begin_frame(window: &mut Window) {
    let now = Instant::now();
    // Every stretch is visited (no short circuit): stale springs are dropped on the way.
    let any = STRETCH.with(|m| {
        m.borrow_mut()
            .values_mut()
            .map(|s| s.animating(now))
            .filter(|a| *a)
            .count()
            > 0
    });
    if any {
        window.request_animation_frame();
    }
}

/// Rubber-band a vertical div scroller tracked by `h`: listens on it (after its own scroll handler).
/// Also enforces gesture ownership (`ui::arbiter`): a vertical gesture is claimed for panes; during a
/// horizontal one (a pager or block owns it) the pane undoes the wobble GPUI applied, back to the offset
/// it was drawn at. Put its children in `content(key)`.
pub fn y_scroller(el: Stateful<Div>, key: &'static str, h: ScrollHandle) -> Stateful<Div> {
    let drawn = h.offset();
    el.on_scroll_wheel(move |ev, window, _| {
        if guard_vertical(ev, &h, drawn.y.into()) && wheel(key, Axis::Y, ev, &h) {
            window.refresh();
        }
    })
}

/// Ownership rule for vertical scrollers: false (and the scroller put back at `drawn_y`) while the gesture
/// is horizontal; a vertical gesture is claimed for panes.
pub fn guard_vertical(ev: &ScrollWheelEvent, s: &dyn Scroller, drawn_y: f32) -> bool {
    match super::arbiter::observe(ev) {
        Some(Axis::X) => {
            if s.pos(Axis::Y).0 != drawn_y {
                s.pin(Axis::Y, drawn_y);
            }
            false
        }
        Some(Axis::Y) => {
            super::arbiter::claim(super::arbiter::Owner::Vertical);
            true
        }
        None => true,
    }
}

/// Wrapper for a scroller's children, translated by the current stretch.
pub fn content(key: &str) -> Div {
    gpui::div().relative().top(shift(key))
}
