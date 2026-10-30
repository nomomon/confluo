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
pub fn gesture_axis(locked: Option<Axis>, phase: TouchPhase, dx: Pixels, dy: Pixels) -> (Axis, Option<Axis>) {
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

thread_local! {
    static LOCKS: std::cell::RefCell<std::collections::HashMap<String, Axis>> = Default::default();
}

/// `gesture_axis` with the lock kept per scroller `key` across events.
pub fn locked_axis(key: &str, phase: TouchPhase, dx: Pixels, dy: Pixels) -> Axis {
    let locked = LOCKS.with(|m| m.borrow().get(key).copied());
    let (axis, keep) = gesture_axis(locked, phase, dx, dy);
    LOCKS.with(|m| match keep {
        Some(a) => m.borrow_mut().insert(key.to_string(), a),
        None => m.borrow_mut().remove(key),
    });
    axis
}

/// Whether a horizontal block owns this event of a sideways gesture. Decided once per gesture, at its
/// first horizontal movement: the block claims it only if it can still scroll that way (`can`), so a swipe
/// that starts at its edge goes on to the page (swipe navigation, panel tabs). Line deltas (mouse wheels)
/// always stay with the block. Returns the answer and the decision to keep.
pub fn claim(kept: Option<bool>, phase: TouchPhase, precise: bool, can: bool) -> (bool, Option<bool>) {
    if !precise {
        return (true, None);
    }
    let kept = if matches!(phase, TouchPhase::Started) { None } else { kept };
    let own = kept.unwrap_or(can);
    (own, if matches!(phase, TouchPhase::Ended) { None } else { Some(own) })
}

thread_local! {
    static CLAIMS: std::cell::RefCell<std::collections::HashMap<String, bool>> = Default::default();
}

/// `claim` kept per block `key`; call for every event locked to x (Ended forgets it).
pub fn claims_x(key: &str, phase: TouchPhase, precise: bool, can: bool) -> bool {
    let kept = CLAIMS.with(|m| m.borrow().get(key).copied());
    let (own, keep) = claim(kept, phase, precise, can);
    CLAIMS.with(|m| match keep {
        Some(k) => m.borrow_mut().insert(key.to_string(), k),
        None => m.borrow_mut().remove(key),
    });
    own
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
        assert_eq!(gesture_axis(None, TouchPhase::Moved, px(0.), px(0.)), (Axis::Y, None));
        assert_eq!(gesture_axis(None, TouchPhase::Moved, px(1.), px(9.)), (Axis::Y, Some(Axis::Y)));
        // Mouse wheels report Moved per notch: Shift+wheel arrives as x and scrolls x.
        assert_eq!(gesture_axis(None, TouchPhase::Moved, px(-20.), px(0.)).0, Axis::X);
    }

    #[test]
    fn horizontal_claim_is_decided_once_per_gesture() {
        // At the edge when it starts: the page gets the whole gesture, even once the block could scroll.
        let (own, k) = claim(None, TouchPhase::Moved, true, false);
        assert!(!own);
        assert_eq!(claim(k, TouchPhase::Moved, true, true), (false, Some(false)));
        // Room to scroll: the block keeps it, also after reaching its edge mid-gesture.
        let (own, k) = claim(None, TouchPhase::Started, true, true);
        assert!(own);
        assert_eq!(claim(k, TouchPhase::Moved, true, false), (true, Some(true)));
        assert_eq!(claim(k, TouchPhase::Ended, true, false), (true, None));
        // A new gesture decides again; mouse wheels always stay with the block.
        assert!(!claim(Some(true), TouchPhase::Started, true, false).0);
        assert_eq!(claim(Some(false), TouchPhase::Moved, false, false), (true, None));
    }
}
