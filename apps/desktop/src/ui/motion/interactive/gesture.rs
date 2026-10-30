//! `SwipeGate`: turns wheel events into a horizontal drag for a `Pager`. Only trackpad gestures with touch
//! phases pass (never mouse wheels), only once the shared per-gesture axis lock says x (so vertical scrolling
//! never starts one), and never from momentum after the fingers lifted.

use gpui::{px, TouchPhase};

use crate::ui::scroll::{gesture_axis, Axis};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drag {
    /// Not a page drag: let the event go on.
    None,
    /// First horizontal movement: begin, then move by the delta.
    Begin(f32),
    Move(f32),
    Release,
}

#[derive(Default)]
pub struct SwipeGate {
    lock: Option<Axis>,
    fingers: bool,
    tracking: bool,
}

impl SwipeGate {
    pub fn feed(&mut self, phase: TouchPhase, precise: bool, dx: f32, dy: f32) -> Drag {
        if !precise {
            return Drag::None;
        }
        let (axis, lock) = gesture_axis(self.lock, phase, px(dx), px(dy));
        self.lock = lock;
        match phase {
            TouchPhase::Started => {
                self.fingers = true;
                self.tracking = false;
                Drag::None
            }
            TouchPhase::Moved if !self.fingers => Drag::None,
            TouchPhase::Moved if self.tracking => Drag::Move(dx),
            TouchPhase::Moved if axis == Axis::X && dx != 0. => {
                self.tracking = true;
                Drag::Begin(dx)
            }
            TouchPhase::Moved => Drag::None,
            TouchPhase::Ended => {
                self.fingers = false;
                if std::mem::take(&mut self.tracking) {
                    Drag::Release
                } else {
                    Drag::None
                }
            }
        }
    }

    pub fn tracking(&self) -> bool {
        self.tracking
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TouchPhase::*;

    #[test]
    fn start_conditions_axis_lock_and_momentum() {
        let mut g = SwipeGate::default();
        // Mouse wheel: never.
        assert_eq!(g.feed(Moved, false, 40., 0.), Drag::None);
        // Vertical gesture that later leans sideways stays vertical.
        g.feed(Started, true, 0., 0.);
        assert_eq!(g.feed(Moved, true, 1., 8.), Drag::None);
        assert_eq!(g.feed(Moved, true, 30., 2.), Drag::None);
        assert_eq!(g.feed(Ended, true, 0., 0.), Drag::None);
        // Momentum after the fingers lifted: never.
        assert_eq!(g.feed(Moved, true, 50., 0.), Drag::None);
        // Sideways: begins, keeps x even when leaning vertical, releases on lift.
        g.feed(Started, true, 0., 0.);
        assert_eq!(g.feed(Moved, true, 10., 1.), Drag::Begin(10.));
        assert_eq!(g.feed(Moved, true, 2., 9.), Drag::Move(2.));
        assert!(g.tracking());
        assert_eq!(g.feed(Ended, true, 0., 0.), Drag::Release);
        assert!(!g.tracking());
    }
}
