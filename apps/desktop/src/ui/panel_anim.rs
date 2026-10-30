//! A side panel's open/close transition: its drawn width springs (`Animated`) toward the open width or 0.
//! Toggles retarget a transition in flight from where it is (interruptible, velocity kept). Reduced
//! motion keeps the width and fades opacity over `FADE` instead.

use std::time::Instant;

use crate::ui::motion::interactive::Animated;
use crate::ui::motion::FADE;

/// Opacity ramp for reduced motion.
#[derive(Clone, Copy, Debug)]
pub struct Fade {
    from: f32,
    to: f32,
    start: Instant,
}

impl Fade {
    fn at(&self, now: Instant) -> (f32, bool) {
        let t = now.saturating_duration_since(self.start).as_secs_f32() * 1000. / FADE.ms as f32;
        let t = t.clamp(0., 1.);
        (self.from + (self.to - self.from) * t, t < 1.)
    }
}

/// What to draw this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelFrame {
    pub width: f32,
    pub opacity: f32,
    pub moving: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PanelAnim {
    pub width: Animated,
    /// A drag collapse in flight: the panel stays open until it settles, then the caller closes it.
    pub closing: bool,
    pub fade: Option<Fade>,
}

impl PanelAnim {
    /// From `from` (drawn now) to `to`, starting the spring at rest.
    pub fn new(from: f32, to: f32, closing: bool, reduced: bool) -> Self {
        let mut width = Animated::new(from);
        width.set(to, Instant::now(), reduced);
        Self {
            width,
            closing,
            fade: None,
        }
    }

    /// A toggle to `open` (the panel state has already changed). `cur` is the transition in flight, if
    /// any; `drawn` the width drawn now; `rest` the open width.
    pub fn toggled(
        cur: Option<Self>,
        drawn: f32,
        rest: f32,
        open: bool,
        reduced: bool,
        now: Instant,
    ) -> Self {
        if reduced {
            let from = cur
                .and_then(|a| a.fade.map(|f| f.at(now).0))
                .unwrap_or(if open { 0. } else { 1. });
            let to = if open { 1. } else { 0. };
            return Self {
                width: Animated::new(rest),
                closing: false,
                fade: Some(Fade {
                    from,
                    to,
                    start: now,
                }),
            };
        }
        let mut width = match cur {
            Some(a) if a.fade.is_none() => a.width,
            _ => Animated::new(drawn),
        };
        width.set(if open { rest } else { 0. }, now, false);
        Self {
            width,
            closing: false,
            fade: None,
        }
    }

    pub fn frame(&mut self, now: Instant) -> PanelFrame {
        let (width, w_moving) = self.width.get(now);
        let (opacity, f_moving) = self.fade.map_or((1., false), |f| f.at(now));
        PanelFrame {
            width,
            opacity,
            moving: w_moving || f_moving,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ms(t: Instant, n: u64) -> Instant {
        t + Duration::from_millis(n)
    }

    fn settle(a: &mut PanelAnim, t: Instant) -> PanelFrame {
        (1..200)
            .map(|i| a.frame(ms(t, i * 16)))
            .find(|f| !f.moving)
            .expect("settles")
    }

    #[test]
    fn opens_from_zero_and_closes_to_zero() {
        let t = Instant::now();
        let mut a = PanelAnim::toggled(None, 0., 360., true, false, t);
        let mid = a.frame(ms(t, 60));
        assert!(mid.moving && mid.width > 0. && mid.width < 360., "{mid:?}");
        assert_eq!(settle(&mut a, t).width, 360.);
        let mut c = PanelAnim::toggled(None, 360., 360., false, false, t);
        assert_eq!(settle(&mut c, t).width, 0.);
    }

    #[test]
    fn interrupt_reverses_from_current_position() {
        let t = Instant::now();
        let mut a = PanelAnim::toggled(None, 0., 360., true, false, t);
        let mid = a.frame(ms(t, 60)).width;
        let mut b = PanelAnim::toggled(Some(a), mid, 360., false, false, ms(t, 60));
        let now = b.frame(ms(t, 60));
        assert!(
            (now.width - mid).abs() < 0.01 && now.moving,
            "no jump: {now:?} {mid}"
        );
        // Still carries the opening velocity a moment later, then heads to 0.
        assert!(b.frame(ms(t, 64)).width > mid - 1.);
        assert_eq!(settle(&mut b, ms(t, 60)).width, 0.);
    }

    #[test]
    fn reduced_motion_fades_at_full_width() {
        let t = Instant::now();
        let mut a = PanelAnim::toggled(None, 0., 360., true, true, t);
        let f = a.frame(ms(t, 50));
        assert_eq!(f.width, 360.);
        assert!(f.opacity > 0. && f.opacity < 1. && f.moving);
        let s = a.frame(ms(t, FADE.ms + 1));
        assert_eq!((s.opacity, s.moving), (1., false));
        // Interrupted fade-out starts from the current opacity.
        let mut a2 = PanelAnim::toggled(None, 0., 360., true, true, t);
        let o = a2.frame(ms(t, 75)).opacity;
        let mut b = PanelAnim::toggled(Some(a2), 360., 360., false, true, ms(t, 75));
        assert!((b.frame(ms(t, 75)).opacity - o).abs() < 0.01);
        assert_eq!(b.frame(ms(t, 400)).opacity, 0.);
    }

    #[test]
    fn drag_collapse_animates_to_zero() {
        let mut p = PanelAnim::new(180., 0., true, false);
        assert_eq!(settle(&mut p, Instant::now()).width, 0.);
        let mut r = PanelAnim::new(180., 0., true, true);
        assert_eq!(r.frame(Instant::now()).width, 0.);
    }
}
