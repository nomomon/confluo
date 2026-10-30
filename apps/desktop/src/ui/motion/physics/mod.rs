//! App-agnostic motion physics (pure, no gpui types): springs, release velocity, Apple's momentum
//! projection, rubber band and inertial decay. `motion::interactive` builds gesture controllers on it.

mod spring;
mod velocity;

#[allow(unused_imports)] // presets are the tuning API
pub use spring::{Spring, SpringParams, BOUNCY, GENTLE, SNAPPY};
pub use velocity::VelocityTracker;

/// Apple's scroll deceleration rate per millisecond (`UIScrollView.DecelerationRate.normal`).
pub const DECELERATION: f32 = 0.998;
/// UIScrollView's rubber band constant.
pub const BAND: f32 = 0.55;

/// Where a flick at `velocity` (units per second) would come to rest: `position + v * d / (1 - d)` with v per ms.
pub fn project(position: f32, velocity: f32, deceleration: f32) -> f32 {
    position + (velocity / 1000.) * deceleration / (1. - deceleration)
}

/// Shown overflow for a drag `overflow` past an edge of a `dimension`-long view:
/// `(1 - 1 / (overflow * c / dimension + 1)) * dimension`, symmetric for negative overflow.
pub fn rubber_band(overflow: f32, dimension: f32, c: f32) -> f32 {
    if dimension <= 0. || !overflow.is_finite() {
        return 0.;
    }
    ((1. - 1. / (overflow.abs() * c / dimension + 1.)) * dimension).copysign(overflow)
}

/// Drag that gives the shown overflow `shown` (grab a band that is springing back without a jump).
pub fn rubber_band_inverse(shown: f32, dimension: f32, c: f32) -> f32 {
    if dimension <= 0. {
        return 0.;
    }
    let s = shown.abs().min(dimension * 0.999);
    ((1. / (1. - s / dimension) - 1.) * dimension / c).copysign(shown)
}

/// Inertial coasting: velocity decays by `d` per millisecond (`v *= d^(dt in ms)`, the same units as
/// `project`, so coasting ends where the projection said).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decay {
    pub v: f32,
    pub d: f32,
}

impl Decay {
    /// Advance `dt` seconds; returns the distance covered.
    pub fn step(&mut self, dt: f32) -> f32 {
        let dt = if dt.is_finite() { dt.clamp(0., 0.1) } else { 0. };
        let moved = self.v * dt;
        self.v *= self.d.powf(dt * 1000.);
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rubber_band_is_monotonic_bounded_and_invertible() {
        let mut last = 0.;
        for i in 1..200 {
            let y = rubber_band(i as f32 * 10., 500., BAND);
            assert!(y > last && y < 500.);
            last = y;
        }
        assert!((rubber_band(1., 500., BAND) - 0.55).abs() < 0.01);
        assert_eq!(rubber_band(-80., 500., BAND), -rubber_band(80., 500., BAND));
        for d in [3., 40., 250., -120.] {
            assert!((rubber_band_inverse(rubber_band(d, 600., BAND), 600., BAND) - d).abs() < 0.05);
        }
        assert_eq!(rubber_band(f32::NAN, 500., BAND), 0.);
    }

    #[test]
    fn projection_and_decay() {
        // A 1000 px/s flick travels 499 px under Apple's deceleration.
        assert!((project(0., 1000., DECELERATION) - 499.).abs() < 1.);
        assert_eq!(project(50., 0., DECELERATION), 50.);
        let mut d = Decay { v: 1000., d: DECELERATION };
        let mut total = 0.;
        for _ in 0..2000 {
            total += d.step(1. / 60.);
        }
        assert!((total - 499.).abs() < 30., "{total}");
    }
}
