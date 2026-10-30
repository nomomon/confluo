//! `Animated`: an `f32` that springs to its target (spatial transitions that are not gestures). Setting a
//! new target mid-flight keeps position and velocity. Reduced motion jumps.

use std::time::Instant;

use crate::ui::motion::physics::{Spring, SpringParams, SNAPPY};

#[derive(Clone, Copy, Debug)]
pub struct Animated {
    target: f32,
    spring: Option<(Spring, Instant)>,
    pub params: SpringParams,
}

impl Animated {
    pub fn new(value: f32) -> Self {
        Self {
            target: value,
            spring: None,
            params: SNAPPY,
        }
    }

    pub fn set(&mut self, target: f32, now: Instant, reduced: bool) {
        if target == self.target {
            return;
        }
        let (x, v) = self
            .spring
            .map_or((self.target, 0.), |(s, _)| (self.target + s.x, s.v));
        self.target = target;
        self.spring = (!reduced).then(|| (Spring::new(self.params, x - target, v), now));
    }

    /// Value at `now`; second: still moving.
    pub fn get(&mut self, now: Instant) -> (f32, bool) {
        let Some((s, last)) = self.spring.as_mut() else {
            return (self.target, false);
        };
        s.step(now.saturating_duration_since(*last).as_secs_f32());
        *last = now;
        if s.is_settled(0.5) {
            self.spring = None;
            return (self.target, false);
        }
        (self.target + s.x, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn springs_to_target_and_retargets_continuously() {
        let t0 = Instant::now();
        let mut a = Animated::new(0.);
        a.set(100., t0, false);
        let (mid, moving) = a.get(t0 + Duration::from_millis(50));
        assert!(moving && mid > 0. && mid < 100.);
        a.set(0., t0 + Duration::from_millis(50), false);
        let (now, _) = a.get(t0 + Duration::from_millis(50));
        assert!((now - mid).abs() < 0.01);
        let mut r = Animated::new(0.);
        r.set(5., t0, true);
        assert_eq!(r.get(t0), (5., false));
    }
}
