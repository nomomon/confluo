//! Per-scroller stretch state machine: fingers past an edge, release, momentum overshoot.

use std::time::Instant;

use gpui::px;

use crate::ui::motion::spring::{rubber_band, rubber_band_inverse, Driven, Spring};
use crate::ui::scroll::{gesture_axis, Axis};

/// Where to hold the scroller after an event the stretch took.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pin {
    Start,
    End,
}

/// One scroller's stretch. Positive pulls the content toward the end (down / right) past the start edge.
#[derive(Default)]
pub struct Stretch {
    pub(super) raw: f32,
    pub(super) shown: f32,
    fingers: bool,
    kicked: bool,
    lock: Option<Axis>,
    spring: Option<Driven>,
}

impl Stretch {
    /// Feed one wheel event along `axis` (`d` is that axis' delta). Returns whether anything changed and
    /// where to hold the scroller (the stretch took the delta, so the content must not move).
    #[allow(clippy::too_many_arguments)]
    pub fn feed(&mut self, axis: Axis, phase: gpui::TouchPhase, precise: bool, dx: f32, dy: f32, at_start: bool, at_end: bool, extent: f32) -> (bool, Option<Pin>) {
        if !precise || crate::ui::motion::reduced_motion() {
            let had = self.shown != 0.;
            *self = Self::default();
            return (had, None);
        }
        let (a, lock) = gesture_axis(self.lock, phase, px(dx), px(dy));
        self.lock = lock;
        let d = if axis == Axis::Y { dy } else { dx };
        match phase {
            gpui::TouchPhase::Started => {
                self.fingers = true;
                self.kicked = false;
                // Grab a band that is still springing back.
                if let Some(s) = self.spring.take() {
                    self.shown = s.spring.x;
                    self.raw = rubber_band_inverse(self.shown, extent);
                }
                (false, None)
            }
            gpui::TouchPhase::Ended => {
                self.fingers = false;
                self.raw = 0.;
                let moving = self.shown != 0.;
                if moving {
                    self.spring = Some(Driven::new(Spring::new(self.shown, 0.)));
                }
                (moving, None)
            }
            gpui::TouchPhase::Moved if a != axis || d == 0. => (false, None),
            gpui::TouchPhase::Moved if self.fingers => {
                let pushing = (at_start && d > 0.) || (at_end && d < 0.);
                if self.raw != 0. {
                    let next = self.raw + d;
                    if next.signum() != self.raw.signum() {
                        self.raw = 0.;
                        self.shown = 0.;
                        return (true, None);
                    }
                    self.raw = next;
                } else if pushing && self.spring.is_none() {
                    self.raw = d;
                } else {
                    return (false, None);
                }
                self.shown = rubber_band(self.raw, extent);
                (true, Some(if self.raw > 0. { Pin::Start } else { Pin::End }))
            }
            // Momentum after the fingers lifted: overshoot once where it hits an edge.
            gpui::TouchPhase::Moved => {
                let pushing = (at_start && d > 0.) || (at_end && d < 0.);
                if self.kicked || self.spring.is_some() || !pushing || d.abs() < 1. {
                    return (false, None);
                }
                self.kicked = true;
                let peak = (d.abs() * 4.).min(extent * 0.08).copysign(d);
                self.spring = Some(Driven::new(Spring::kick(peak)));
                (true, None)
            }
        }
    }

    /// A spring is running; one not stepped for 250ms (its pane is gone) is dropped.
    pub fn animating(&mut self, now: Instant) -> bool {
        if self.spring.as_ref().is_some_and(|s| now.saturating_duration_since(s.last) > std::time::Duration::from_millis(250)) {
            self.spring = None;
            self.shown = 0.;
        }
        self.spring.is_some()
    }

    /// The translate to draw now; steps the spring. Second value: still animating.
    pub fn shown(&mut self, now: Instant) -> (f32, bool) {
        if let Some(s) = self.spring.as_mut() {
            let x = s.advance(now);
            if s.spring.settled() {
                self.spring = None;
                self.shown = 0.;
                return (0., false);
            }
            self.shown = x;
            return (x, true);
        }
        (self.shown, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use gpui::TouchPhase::*;

    #[test]
    fn stretch_pulls_with_resistance_and_reverses() {
        let mut s = Stretch::default();
        assert_eq!(s.feed(Axis::Y, Started, true, 0., 0., true, false, 400.), (false, None));
        // Not at the edge: nothing.
        assert_eq!(s.feed(Axis::Y, Moved, true, 0., 10., false, false, 400.), (false, None));
        assert_eq!(s.feed(Axis::Y, Moved, true, 0., 20., true, false, 400.), (true, Some(Pin::Start)));
        assert!(s.shown > 0. && s.shown < 20.);
        // Pushing back first unwinds the stretch, holding the content at the edge.
        assert_eq!(s.feed(Axis::Y, Moved, true, 0., -10., true, false, 400.).1, Some(Pin::Start));
        assert_eq!(s.feed(Axis::Y, Moved, true, 0., -15., true, false, 400.), (true, None));
        assert_eq!(s.shown, 0.);
        // Bottom edge stretches the other way; release springs back to 0.
        s.feed(Axis::Y, Moved, true, 0., -30., false, true, 400.);
        assert!(s.shown < 0.);
        assert!(s.feed(Axis::Y, Ended, true, 0., 0., false, true, 400.).0);
        let t = Instant::now();
        let mut done = false;
        for i in 1..60 {
            if !s.shown(t + std::time::Duration::from_millis(i * 16)).1 {
                done = true;
                break;
            }
        }
        assert!(done && s.shown == 0.);
    }

    #[test]
    fn wheel_mouse_and_cross_axis_never_stretch() {
        let mut s = Stretch::default();
        assert_eq!(s.feed(Axis::Y, Moved, false, 0., 40., true, false, 400.), (false, None));
        s.feed(Axis::Y, Started, true, 0., 0., true, false, 400.);
        // A sideways gesture locks to x: no vertical stretch even when it later leans down.
        s.feed(Axis::Y, Moved, true, 12., 1., true, false, 400.);
        assert_eq!(s.feed(Axis::Y, Moved, true, 1., 20., true, false, 400.), (false, None));
    }

    #[test]
    fn momentum_overshoots_once() {
        let mut s = Stretch::default();
        s.feed(Axis::Y, Started, true, 0., 0., false, false, 400.);
        s.feed(Axis::Y, Moved, true, 0., -5., false, false, 400.);
        s.feed(Axis::Y, Ended, true, 0., 0., false, false, 400.);
        assert!(s.feed(Axis::Y, Moved, true, 0., -8., false, true, 400.).0);
        assert!(!s.feed(Axis::Y, Moved, true, 0., -6., false, true, 400.).0);
    }
}
