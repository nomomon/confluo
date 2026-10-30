//! Damped spring, SwiftUI-style parameters (response, damping fraction) converted to stiffness and
//! damping. Integrated with semi-implicit Euler in fixed sub-steps, so any frame time is stable.

use std::f32::consts::PI;

/// Physical parameters: `stiffness` k, `damping` c, `mass` m.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringParams {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
}

impl SpringParams {
    /// `response`: period of the undamped oscillation (s); `damping_fraction`: 1 critically damped, < 1 overshoots.
    pub fn new(response: f32, damping_fraction: f32) -> Self {
        let mass = 1.;
        let w = 2. * PI / response.max(0.01);
        Self {
            stiffness: w * w * mass,
            damping: 2. * damping_fraction * w * mass,
            mass,
        }
    }
}

/// Quick and settled, no overshoot: gestures snapping to a page, rubber band spring-back.
pub const SNAPPY: SpringParams = SpringParams {
    stiffness: 438.65,
    damping: 41.89,
    mass: 1.,
}; // response 0.3, fraction 1
/// Slight overshoot (fraction 0.85): playful, for direct manipulation that lands on a target.
pub const BOUNCY: SpringParams = SpringParams {
    stiffness: 322.27,
    damping: 30.52,
    mass: 1.,
}; // response 0.35, fraction 0.85
/// Slow and smooth (response 0.5, critically damped): large panels.
pub const GENTLE: SpringParams = SpringParams {
    stiffness: 157.91,
    damping: 25.13,
    mass: 1.,
};

/// Largest integration step (s).
const SUB_STEP: f32 = 1. / 240.;
/// Longest frame gap integrated at once (s); a stalled window does not jump past the curve.
const MAX_DT: f32 = 0.1;

/// A spring pulling `x` (the displacement from its target) to 0, moving at `v` (units per second).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub params: SpringParams,
    pub x: f32,
    pub v: f32,
}

impl Spring {
    /// Start at displacement `x` with velocity `v` (velocity handoff from a gesture).
    pub fn new(params: SpringParams, x: f32, v: f32) -> Self {
        let ok = |f: f32| if f.is_finite() { f } else { 0. };
        Self {
            params,
            x: ok(x),
            v: ok(v),
        }
    }

    pub fn step(&mut self, dt: f32) {
        let mut left = if dt.is_finite() {
            dt.clamp(0., MAX_DT)
        } else {
            0.
        };
        let SpringParams {
            stiffness: k,
            damping: c,
            mass: m,
        } = self.params;
        while left > 0. {
            let h = left.min(SUB_STEP);
            let a = (-k * self.x - c * self.v) / m;
            self.v += a * h;
            self.x += self.v * h;
            left -= h;
        }
    }

    /// Within `eps` of the target and slow.
    pub fn is_settled(&self, eps: f32) -> bool {
        self.x.abs() < eps && self.v.abs() < eps * 20.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_match_their_response() {
        for (p, r, f) in [(SNAPPY, 0.3, 1.), (BOUNCY, 0.35, 0.85), (GENTLE, 0.5, 1.)] {
            let q = SpringParams::new(r, f);
            assert!(
                (p.stiffness - q.stiffness).abs() < 0.1 && (p.damping - q.damping).abs() < 0.1,
                "{p:?} vs {q:?}"
            );
        }
    }

    #[test]
    fn critically_damped_settles_without_crossing() {
        let mut s = Spring::new(SNAPPY, 100., 0.);
        let mut t = 0.;
        while !s.is_settled(0.5) {
            s.step(1. / 60.);
            assert!(s.x > -0.01);
            t += 1. / 60.;
            assert!(t < 2.);
        }
        assert!(
            (0.2..0.6).contains(&t),
            "settles in about 300-400ms, took {t}"
        );
    }

    #[test]
    fn extreme_dt_never_goes_nan() {
        let mut s = Spring::new(SNAPPY, 1e6, -1e7);
        for dt in [0., 1e-9, 1., 1e9, f32::INFINITY, f32::NAN, -1.] {
            s.step(dt);
            assert!(s.x.is_finite() && s.v.is_finite());
        }
        assert!(Spring::new(SNAPPY, f32::NAN, f32::INFINITY).x == 0.);
    }

    #[test]
    fn velocity_handoff_is_continuous() {
        // Starting with the gesture's velocity, the first frame moves the way the fingers did.
        let mut s = Spring::new(SNAPPY, -200., 1500.);
        let x0 = s.x;
        s.step(1. / 120.);
        // (velocity plus the spring's own pull over one frame, no jump)
        assert!(s.x - x0 > 1500. / 120. && s.x - x0 < 1500. / 120. * 2.);
        // Bouncy overshoots a little, then settles.
        let mut b = Spring::new(BOUNCY, 100., 0.);
        let mut low: f32 = 0.;
        for _ in 0..240 {
            b.step(1. / 120.);
            low = low.min(b.x);
        }
        assert!(low < 0. && low > -10. && b.is_settled(0.5));
    }
}
