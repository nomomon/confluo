//! `ElasticScroll`: rubber-band overscroll of one scroller axis. Fingers past an edge stretch it with
//! resistance; release springs back (SNAPPY, critically damped); momentum reaching an edge hands its
//! velocity to the spring, which overshoots a little and returns. Mouse wheels and reduced motion hard-stop.

use std::time::Instant;

use gpui::{px, TouchPhase};

use crate::ui::motion::physics::{
    rubber_band, rubber_band_inverse, Spring, VelocityTracker, BAND, SNAPPY,
};
use crate::ui::scroll::{gesture_axis, Axis};

/// Where to hold the scroller after an event the stretch took (the content must not move).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pin {
    Start,
    End,
}

/// Positive stretch pulls the content toward the end (down / right) past the start edge.
#[derive(Default)]
pub struct ElasticScroll {
    raw: f32,
    shown: f32,
    fingers: bool,
    handed_off: bool,
    lock: Option<Axis>,
    spring: Option<(Spring, Instant)>,
    momentum: VelocityTracker,
    momentum_pos: f32,
    epoch: Option<Instant>,
}

impl ElasticScroll {
    /// Feed one wheel event for `axis`. `at_start` / `at_end`: the scroller sits at that edge; `extent` its length.
    /// Returns whether anything changed and where to hold the scroller.
    #[allow(clippy::too_many_arguments)]
    pub fn feed(
        &mut self,
        axis: Axis,
        phase: TouchPhase,
        precise: bool,
        reduced: bool,
        dx: f32,
        dy: f32,
        at_start: bool,
        at_end: bool,
        extent: f32,
        now: Instant,
    ) -> (bool, Option<Pin>) {
        if !precise || reduced {
            let had = self.shown != 0.;
            *self = Self::default();
            return (had, None);
        }
        let (a, lock) = gesture_axis(self.lock, phase, px(dx), px(dy));
        self.lock = lock;
        let d = if axis == Axis::Y { dy } else { dx };
        let pushing = (at_start && d > 0.) || (at_end && d < 0.);
        let epoch = *self.epoch.get_or_insert(now);
        match phase {
            TouchPhase::Started => {
                self.fingers = true;
                self.handed_off = false;
                self.momentum.reset();
                // Grab a band that is still springing back, without a jump.
                if let Some((s, _)) = self.spring.take() {
                    self.shown = s.x;
                    self.raw = rubber_band_inverse(self.shown, extent, BAND);
                }
                (false, None)
            }
            TouchPhase::Ended => {
                self.fingers = false;
                self.raw = 0.;
                let moving = self.shown != 0.;
                if moving {
                    self.spring = Some((Spring::new(SNAPPY, self.shown, 0.), now));
                }
                (moving, None)
            }
            TouchPhase::Moved if a != axis || d == 0. => (false, None),
            TouchPhase::Moved if self.fingers => {
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
                self.shown = rubber_band(self.raw, extent, BAND);
                (
                    true,
                    Some(if self.raw > 0. { Pin::Start } else { Pin::End }),
                )
            }
            // Momentum: track its speed; where it reaches an edge, hand that velocity to the spring.
            TouchPhase::Moved => {
                self.momentum_pos += d;
                self.momentum.push(
                    now.saturating_duration_since(epoch).as_secs_f32(),
                    self.momentum_pos,
                );
                if self.handed_off || self.spring.is_some() || !pushing {
                    return (false, None);
                }
                self.handed_off = true;
                let v = self
                    .momentum
                    .velocity(now.saturating_duration_since(epoch).as_secs_f32());
                // Cap the overshoot near 8% of the extent: a critically damped spring peaks at v / (w e).
                let w = (SNAPPY.stiffness / SNAPPY.mass).sqrt();
                let cap = extent * 0.08 * w * std::f32::consts::E;
                let v = if v * d > 0. {
                    v.clamp(-cap, cap)
                } else {
                    (d * 60.).clamp(-cap, cap)
                };
                self.spring = Some((Spring::new(SNAPPY, 0., v), now));
                (true, None)
            }
        }
    }

    /// The translate to draw at `now` (steps the spring); second: still animating.
    pub fn shown(&mut self, now: Instant) -> (f32, bool) {
        if let Some((s, last)) = self.spring.as_mut() {
            s.step(now.saturating_duration_since(*last).as_secs_f32());
            *last = now;
            if s.is_settled(0.5) {
                self.spring = None;
                self.shown = 0.;
                return (0., false);
            }
            self.shown = s.x;
            return (s.x, true);
        }
        (self.shown, false)
    }

    /// A spring is running; one not stepped for 250ms (its pane is gone) is dropped.
    pub fn animating(&mut self, now: Instant) -> bool {
        if self
            .spring
            .is_some_and(|(_, last)| now.saturating_duration_since(last).as_millis() > 250)
        {
            self.spring = None;
            self.shown = 0.;
        }
        self.spring.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use TouchPhase::*;

    fn feed(
        s: &mut ElasticScroll,
        phase: TouchPhase,
        dy: f32,
        at_start: bool,
        at_end: bool,
    ) -> (bool, Option<Pin>) {
        s.feed(
            Axis::Y,
            phase,
            true,
            false,
            0.,
            dy,
            at_start,
            at_end,
            400.,
            Instant::now(),
        )
    }

    #[test]
    fn stretch_pulls_with_resistance_reverses_and_springs_back() {
        let mut s = ElasticScroll::default();
        assert_eq!(feed(&mut s, Started, 0., true, false), (false, None));
        assert_eq!(feed(&mut s, Moved, 10., false, false), (false, None));
        assert_eq!(
            feed(&mut s, Moved, 20., true, false),
            (true, Some(Pin::Start))
        );
        assert!(s.shown > 0. && s.shown < 20.);
        assert_eq!(feed(&mut s, Moved, -10., true, false).1, Some(Pin::Start));
        assert_eq!(feed(&mut s, Moved, -15., true, false), (true, None));
        assert_eq!(s.shown, 0.);
        feed(&mut s, Moved, -30., false, true);
        assert!(s.shown < 0.);
        assert!(feed(&mut s, Ended, 0., false, true).0);
        let t = Instant::now();
        let done = (1..80).any(|i| !s.shown(t + Duration::from_millis(i * 16)).1);
        assert!(done && s.shown == 0.);
    }

    #[test]
    fn mouse_wheel_reduced_motion_and_cross_axis_never_stretch() {
        let mut s = ElasticScroll::default();
        assert_eq!(
            s.feed(
                Axis::Y,
                Moved,
                false,
                false,
                0.,
                40.,
                true,
                false,
                400.,
                Instant::now()
            ),
            (false, None)
        );
        assert_eq!(
            s.feed(
                Axis::Y,
                Moved,
                true,
                true,
                0.,
                40.,
                true,
                false,
                400.,
                Instant::now()
            ),
            (false, None)
        );
        feed(&mut s, Started, 0., true, false);
        s.feed(
            Axis::Y,
            Moved,
            true,
            false,
            12.,
            1.,
            true,
            false,
            400.,
            Instant::now(),
        );
        assert_eq!(
            s.feed(
                Axis::Y,
                Moved,
                true,
                false,
                1.,
                20.,
                true,
                false,
                400.,
                Instant::now()
            ),
            (false, None)
        );
    }

    #[test]
    fn momentum_hands_off_velocity_once() {
        let mut s = ElasticScroll::default();
        feed(&mut s, Started, 0., false, false);
        feed(&mut s, Moved, -5., false, false);
        feed(&mut s, Ended, 0., false, false);
        let t0 = Instant::now();
        for i in 0..4 {
            s.feed(
                Axis::Y,
                Moved,
                true,
                false,
                0.,
                -20.,
                false,
                false,
                400.,
                t0 + Duration::from_millis(i * 8),
            );
        }
        assert!(
            s.feed(
                Axis::Y,
                Moved,
                true,
                false,
                0.,
                -20.,
                false,
                true,
                400.,
                t0 + Duration::from_millis(40)
            )
            .0
        );
        assert!(
            !s.feed(
                Axis::Y,
                Moved,
                true,
                false,
                0.,
                -18.,
                false,
                true,
                400.,
                t0 + Duration::from_millis(48)
            )
            .0
        );
        // It overshoots past the end (negative), bounded, then settles.
        let mut low: f32 = 0.;
        for i in 1..120 {
            low = low.min(s.shown(t0 + Duration::from_millis(40 + i * 8)).0);
        }
        assert!(low < 0. && low > -40.);
    }
}
