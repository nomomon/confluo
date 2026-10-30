//! `ElasticScroll`: rubber-band overscroll of one scroller axis. Fingers past an edge stretch it with
//! resistance; release springs back (SNAPPY, critically damped); momentum reaching an edge hands its
//! velocity to the spring, which overshoots a little and returns. Mouse wheels and reduced motion hard-stop.

use std::time::Instant;

use gpui::{px, TouchPhase};

use crate::ui::motion::physics::{
    rubber_band, rubber_band_inverse, Spring, VelocityTracker, BAND, SNAPPY,
};
use crate::ui::scroll::{gesture_axis, Axis};

/// Largest spring start velocity: a critically damped spring from rest peaks at `v / (w e)`, so this keeps
/// any overshoot within 8% of the extent.
fn overshoot_cap(extent: f32) -> f32 {
    let w = (SNAPPY.stiffness / SNAPPY.mass).sqrt();
    extent * 0.08 * w * std::f32::consts::E
}

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
                self.momentum_pos = 0.;
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
                    // Released while stretched: the release velocity goes to the spring once, and the rest
                    // of this gesture's momentum is swallowed (it would otherwise stretch a second time).
                    let t = now.saturating_duration_since(epoch).as_secs_f32();
                    let v = (self.momentum.velocity(t) * BAND)
                        .clamp(-overshoot_cap(extent), overshoot_cap(extent));
                    self.spring = Some((Spring::new(SNAPPY, self.shown, v), now));
                    self.handed_off = true;
                }
                (moving, None)
            }
            TouchPhase::Moved if a != axis || d == 0. => (false, None),
            TouchPhase::Moved if self.fingers => {
                self.momentum_pos += d;
                let t = now.saturating_duration_since(epoch).as_secs_f32();
                self.momentum.push(t, self.momentum_pos);
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
                let cap = overshoot_cap(extent);
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

    /// Count excursions: separate runs of frames with a nonzero stretch.
    fn excursions(
        s: &mut ElasticScroll,
        t0: Instant,
        from_ms: u64,
        frames: u64,
        peak: &mut f32,
    ) -> usize {
        let (mut n, mut was) = (0, s.shown != 0.);
        for i in 1..=frames {
            let (x, _) = s.shown(t0 + Duration::from_millis(from_ms + i * 8));
            *peak = peak.max(x.abs());
            if x != 0. && !was {
                n += 1;
            }
            was = x != 0.;
        }
        n
    }

    #[test]
    fn release_past_edge_then_momentum_stretches_once() {
        let mut s = ElasticScroll::default();
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        s.feed(
            Axis::Y,
            Started,
            true,
            false,
            0.,
            0.,
            true,
            false,
            400.,
            at(0),
        );
        for i in 1..=10 {
            s.feed(
                Axis::Y,
                Moved,
                true,
                false,
                0.,
                12.,
                true,
                false,
                400.,
                at(i * 8),
            );
        }
        assert!(s.shown > 0.);
        s.feed(
            Axis::Y,
            Ended,
            true,
            false,
            0.,
            0.,
            true,
            false,
            400.,
            at(88),
        );
        // 20 momentum events pushing past the same edge, interleaved with frames: swallowed.
        let mut peak: f32 = 0.;
        let mut second = 0;
        for i in 1..=20u64 {
            let (changed, pin) = s.feed(
                Axis::Y,
                Moved,
                true,
                false,
                0.,
                10. - i as f32 * 0.4,
                true,
                false,
                400.,
                at(88 + i * 16),
            );
            assert!(!changed && pin.is_none());
            let (x, _) = s.shown(at(88 + i * 16 + 1));
            peak = peak.max(x);
            if i > 1 && x == 0. && s.shown(at(88 + i * 16 + 2)).0 != 0. {
                second += 1;
            }
        }
        // It settles once and never stretches again.
        assert_eq!(excursions(&mut s, t0, 88 + 20 * 16, 100, &mut peak), 0);
        assert_eq!(second, 0);
        assert_eq!(s.shown, 0.);
        assert!(peak < 400. * 0.2);
    }

    #[test]
    fn momentum_into_edge_overshoots_once_bounded() {
        let mut s = ElasticScroll::default();
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        s.feed(
            Axis::Y,
            Started,
            true,
            false,
            0.,
            0.,
            false,
            false,
            400.,
            at(0),
        );
        s.feed(
            Axis::Y,
            Moved,
            true,
            false,
            0.,
            -30.,
            false,
            false,
            400.,
            at(8),
        );
        s.feed(
            Axis::Y,
            Ended,
            true,
            false,
            0.,
            0.,
            false,
            false,
            400.,
            at(16),
        );
        // In-bounds momentum, then it reaches the end and keeps pushing.
        for i in 0..6 {
            s.feed(
                Axis::Y,
                Moved,
                true,
                false,
                0.,
                -25.,
                false,
                false,
                400.,
                at(24 + i * 8),
            );
        }
        let mut kicks = 0;
        let mut peak: f32 = 0.;
        for i in 0..20u64 {
            if s.feed(
                Axis::Y,
                Moved,
                true,
                false,
                0.,
                -20. + i as f32,
                false,
                true,
                400.,
                at(72 + i * 16),
            )
            .0
            {
                kicks += 1;
            }
            peak = peak.max(s.shown(at(72 + i * 16 + 1)).0.abs());
        }
        excursions(&mut s, t0, 72 + 20 * 16, 120, &mut peak);
        assert_eq!(kicks, 1);
        assert!(peak > 0. && peak <= 400. * 0.08 + 0.5, "{peak}");
        assert_eq!(s.shown, 0.);
    }
}
