//! Physics for direct manipulation: Apple-like rubber-band damping and a critically damped spring
//! (closed form, so any frame rate gives the same curve and it never oscillates).

use std::time::Instant;

/// Resistance constant of the rubber band (UIScrollView uses 0.55).
const BAND: f32 = 0.55;

/// Shown stretch for a finger distance `d` past an edge of a pane `extent` long:
/// `d * (1 - 1 / (d * 0.55 / extent + 1))` style, approaching `extent` but never reaching it.
pub fn rubber_band(d: f32, extent: f32) -> f32 {
    if extent <= 0. {
        return 0.;
    }
    let shown = (1. - 1. / (d.abs() * BAND / extent + 1.)) * extent;
    shown.copysign(d)
}

/// Finger distance that gives the stretch `shown` (grabbing a band that is springing back).
pub fn rubber_band_inverse(shown: f32, extent: f32) -> f32 {
    if extent <= 0. {
        return 0.;
    }
    let s = shown.abs().min(extent * 0.999);
    ((1. / (1. - s / extent) - 1.) * extent / BAND).copysign(shown)
}

/// Stiffness (rad/s) of the settle spring: a typical stretch is back within ~350ms.
pub const SETTLE_OMEGA: f32 = 20.;

/// Critically damped spring toward 0: `x` is the displacement, `v` its velocity (per second).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub x: f32,
    pub v: f32,
    pub omega: f32,
}

impl Spring {
    pub fn new(x: f32, v: f32) -> Self {
        Self { x, v, omega: SETTLE_OMEGA }
    }

    /// Advance by `dt` seconds: `x(t) = (x0 + (v0 + w x0) t) e^(-w t)`.
    pub fn step(&mut self, dt: f32) {
        let w = self.omega;
        let c = self.v + w * self.x;
        let e = (-w * dt).exp();
        self.x = (self.x + c * dt) * e;
        self.v = (self.v - w * c * dt) * e;
        if self.settled() {
            self.x = 0.;
            self.v = 0.;
        }
    }

    /// Under half a pixel and slow: done.
    pub fn settled(&self) -> bool {
        self.x.abs() < 0.5 && self.v.abs() < 20.
    }

    /// Spring that starts at rest and peaks at `peak` (momentum hitting an edge): `v0 = peak * w * e`.
    pub fn kick(peak: f32) -> Self {
        Self::new(0., peak * SETTLE_OMEGA * std::f32::consts::E)
    }
}

/// A spring driven by wall-clock frames.
#[derive(Clone, Copy, Debug)]
pub struct Driven {
    pub spring: Spring,
    pub last: Instant,
}

impl Driven {
    pub fn new(spring: Spring) -> Self {
        Self { spring, last: Instant::now() }
    }

    /// Step to `now` (a long gap counts as one 1/30 s frame, so a stalled window does not jump).
    pub fn advance(&mut self, now: Instant) -> f32 {
        let dt = now.saturating_duration_since(self.last).as_secs_f32().min(1. / 30.);
        self.last = now;
        self.spring.step(dt);
        self.spring.x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rubber_band_damps_and_inverts() {
        assert_eq!(rubber_band(0., 500.), 0.);
        // Near 1:1 at first (slope 0.55), then ever stiffer, never past the extent.
        assert!((rubber_band(1., 500.) - 0.55).abs() < 0.01);
        assert!(rubber_band(500., 500.) < 200.);
        assert!(rubber_band(1e6, 500.) < 500.);
        assert!(rubber_band(-100., 500.) < 0.);
        for d in [3., 40., 250., -120.] {
            assert!((rubber_band_inverse(rubber_band(d, 600.), 600.) - d).abs() < 0.01);
        }
    }

    #[test]
    fn spring_settles_without_overshoot() {
        let mut s = Spring::new(80., 0.);
        let mut t = 0.;
        while !s.settled() {
            s.step(1. / 120.);
            assert!(s.x >= 0., "critically damped: never crosses zero");
            t += 1. / 120.;
            assert!(t < 1.);
        }
        assert!((0.25..0.45).contains(&t), "settles in about 300-400ms, took {t}");
        // A kick peaks near the asked height and comes back.
        let mut k = Spring::kick(30.);
        let mut peak: f32 = 0.;
        for _ in 0..120 {
            k.step(1. / 120.);
            peak = peak.max(k.x);
        }
        assert!((peak - 30.).abs() < 1.5 && k.settled());
    }
}
