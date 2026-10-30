//! Scroll axis rules shared by every scroller.
//!
//! GPUI's built-in handler turns a horizontal swipe into vertical scrolling on a y-only scroller
//! unless `restrict_scroll_to_axis` is set, and an inner horizontal scroller does not stop the event,
//! so a sideways swipe on a wide table also moved the pane around it. Panes use `y_only`; horizontal
//! blocks (`markdown::table::hscroll`) lock each trackpad gesture to its dominant axis, keep the x part
//! (also at their edges) and let the y part bubble to the pane.

use gpui::{Overflow, Pixels, Styled, TouchPhase};

/// Vertical scroller that ignores horizontal wheel deltas instead of mapping them to y.
pub fn y_only<E: Styled>(mut e: E) -> E {
    let s = e.style();
    s.overflow.y = Some(Overflow::Scroll);
    s.restrict_scroll_to_axis = Some(true);
    e
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

/// The axis that wins one wheel event: x only when it moves strictly more than y.
pub fn dominant(dx: Pixels, dy: Pixels) -> Axis {
    if dx.abs() > dy.abs() {
        Axis::X
    } else {
        Axis::Y
    }
}

/// Axis for this event of a gesture: decided by the first event that moves (or a new gesture's
/// start), then kept until the gesture ends, so a diagonal swipe never drifts onto the other axis.
/// Returns the axis and the lock to keep for the next event.
pub fn gesture_axis(
    locked: Option<Axis>,
    phase: TouchPhase,
    dx: Pixels,
    dy: Pixels,
) -> (Axis, Option<Axis>) {
    let moving = dx != Pixels::ZERO || dy != Pixels::ZERO;
    let axis = match (phase, locked) {
        (TouchPhase::Started, _) | (_, None) => dominant(dx, dy),
        (_, Some(a)) => a,
    };
    let keep = match phase {
        TouchPhase::Ended => None,
        // An empty first event (or a new gesture's empty start) decides nothing yet.
        _ if !moving && (locked.is_none() || matches!(phase, TouchPhase::Started)) => None,
        _ => Some(axis),
    };
    (axis, keep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn dominant_axis_and_gesture_lock() {
        assert_eq!(dominant(px(5.), px(3.)), Axis::X);
        assert_eq!(dominant(px(-5.), px(3.)), Axis::X);
        assert_eq!(dominant(px(3.), px(3.)), Axis::Y);
        assert_eq!(dominant(px(0.), px(-1.)), Axis::Y);
        // A sideways gesture stays horizontal even when a later event leans vertical.
        let (a, lock) = gesture_axis(None, TouchPhase::Started, px(-8.), px(2.));
        assert_eq!((a, lock), (Axis::X, Some(Axis::X)));
        let (a, lock) = gesture_axis(lock, TouchPhase::Moved, px(-1.), px(6.));
        assert_eq!((a, lock), (Axis::X, Some(Axis::X)));
        let (a, lock) = gesture_axis(lock, TouchPhase::Ended, px(0.), px(0.));
        assert_eq!((a, lock), (Axis::X, None));
        // The next gesture decides again; an empty first event does not lock.
        assert_eq!(
            gesture_axis(None, TouchPhase::Moved, px(0.), px(0.)),
            (Axis::Y, None)
        );
        assert_eq!(
            gesture_axis(None, TouchPhase::Moved, px(1.), px(9.)),
            (Axis::Y, Some(Axis::Y))
        );
        // Mouse wheels report Moved per notch: Shift+wheel arrives as x and scrolls x.
        assert_eq!(
            gesture_axis(None, TouchPhase::Moved, px(-20.), px(0.)).0,
            Axis::X
        );
    }
}
