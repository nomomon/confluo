//! Ending pointer drags reliably. Element `on_mouse_up` only fires while the pointer is over that element,
//! so a release over other content (a collapsing panel slides out from under the pointer) or outside the
//! window left drags stuck to the pointer. `watch` registers window-level listeners for the drag's lifetime:
//! any left release, or a move with no button pressed (a release the window never saw), ends it.

use gpui::{canvas, prelude::*, App, MouseButton, MouseMoveEvent, MouseUpEvent, Window};

/// Something that happened while a drag is live.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Signal {
    /// Pointer moved; `pressed` is whether the dragging (left) button is still down.
    Move { pressed: bool },
    /// Left button released, wherever the pointer is.
    Release,
    /// The window lost focus.
    Blur,
    /// Esc pressed.
    Escape,
}

/// Whether `s` ends the drag.
pub fn ends(s: Signal) -> bool {
    !matches!(s, Signal::Move { pressed: true })
}

/// An invisible element that, while rendered, calls `on_end` on the first window-level signal ending the
/// drag. Render it only while the drag is live.
pub fn watch(on_end: impl Fn(&mut Window, &mut App) + 'static) -> impl IntoElement {
    let on_end = std::rc::Rc::new(on_end);
    canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            let end = on_end.clone();
            window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                if phase.capture() && ev.button == MouseButton::Left && ends(Signal::Release) {
                    end(window, cx)
                }
            });
            let end = on_end.clone();
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, cx| {
                let pressed = ev.pressed_button == Some(MouseButton::Left);
                if phase.capture() && ends(Signal::Move { pressed }) {
                    end(window, cx)
                }
            });
        },
    )
    .absolute()
    .size_0()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_pressed_move_keeps_the_drag() {
        assert!(!ends(Signal::Move { pressed: true }));
        assert!(
            ends(Signal::Move { pressed: false }),
            "release the window never saw"
        );
        assert!(
            ends(Signal::Release),
            "release over other content or outside"
        );
        assert!(ends(Signal::Blur), "focus loss");
        assert!(ends(Signal::Escape));
    }
}
