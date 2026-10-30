//! Release velocity of a gesture from its last ~100ms of samples.

/// Samples kept and the window they are weighed over (s).
const KEEP: usize = 20;
const WINDOW: f32 = 0.1;

/// Collects `(time s, position)` samples; `velocity` weighs the recent ones more.
#[derive(Clone, Debug, Default)]
pub struct VelocityTracker {
    samples: Vec<(f32, f32)>,
}

impl VelocityTracker {
    pub fn reset(&mut self) {
        self.samples.clear();
    }

    pub fn push(&mut self, t: f32, position: f32) {
        if !t.is_finite() || !position.is_finite() {
            return;
        }
        if self.samples.len() == KEEP {
            self.samples.remove(0);
        }
        self.samples.push((t, position));
    }

    /// Units per second at `now`: segment speeds in the last 100ms, newer segments weighing more.
    /// Fingers that rested before lifting give 0.
    pub fn velocity(&self, now: f32) -> f32 {
        let Some(&(last_t, _)) = self.samples.last() else { return 0. };
        if now - last_t > WINDOW * 0.8 {
            return 0.;
        }
        let recent: Vec<_> = self.samples.iter().filter(|(t, _)| last_t - t <= WINDOW).collect();
        let (mut sum, mut weight) = (0., 0.);
        for (i, w) in recent.windows(2).enumerate() {
            let dt = w[1].0 - w[0].0;
            if dt <= 1e-4 {
                continue;
            }
            let k = (i + 1) as f32;
            sum += (w[1].1 - w[0].1) / dt * k;
            weight += k;
        }
        if weight == 0. {
            0.
        } else {
            sum / weight
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steady_drag_rest_and_window() {
        let mut v = VelocityTracker::default();
        for i in 0..10 {
            v.push(i as f32 * 0.01, i as f32 * 10.);
        }
        assert!((v.velocity(0.09) - 1000.).abs() < 1.);
        // Resting 200ms before release: no flick.
        assert_eq!(v.velocity(0.3), 0.);
        // Old samples outside the window do not count.
        v.push(0.1, 100.);
        v.push(0.2, 100.);
        v.push(0.21, 100.);
        assert!(v.velocity(0.21).abs() < 1.);
        assert_eq!(VelocityTracker::default().velocity(0.), 0.);
    }
}
