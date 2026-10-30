//! `Pager`: a value dragged between N snap points (pages). Drags follow 1:1 inside the allowed pages and
//! rubber-band past them; release projects the position with the release velocity (Apple's deceleration)
//! and springs to the chosen page starting at that velocity. A new drag mid-animation grabs the current
//! position with no jump. Reduced motion snaps without animating.

use std::time::Instant;

use crate::ui::motion::physics::{
    project, rubber_band, rubber_band_inverse, Spring, SpringParams, VelocityTracker, BAND,
    DECELERATION, SNAPPY,
};

pub struct Pager {
    snaps: Vec<f32>,
    lo: usize,
    hi: usize,
    /// Target (or resting) page.
    page: usize,
    pos: f32,
    raw: f32,
    from: usize,
    dragging: bool,
    spring: Option<Spring>,
    tracker: VelocityTracker,
    epoch: Instant,
    last: Instant,
    pub params: SpringParams,
    /// Share of the way to the neighbor page past which a slow release goes there (0.5 = nearest).
    pub commit: f32,
    pub reduced: bool,
}

impl Pager {
    pub fn new(snaps: Vec<f32>, page: usize) -> Self {
        let now = Instant::now();
        let pos = snaps.get(page).copied().unwrap_or(0.);
        let hi = snaps.len().saturating_sub(1);
        Self {
            snaps,
            lo: 0,
            hi,
            page,
            pos,
            raw: pos,
            from: page,
            dragging: false,
            spring: None,
            tracker: VelocityTracker::default(),
            epoch: now,
            last: now,
            params: SNAPPY,
            commit: 0.5,
            reduced: false,
        }
    }

    /// Snap positions (sorted either way) and the pages a drag may reach (`lo..=hi`); call each frame (sizes change).
    pub fn set_snaps(&mut self, snaps: Vec<f32>, lo: usize, hi: usize) {
        let resting = !self.moving();
        self.snaps = snaps;
        self.hi = hi.min(self.snaps.len().saturating_sub(1));
        self.lo = lo.min(self.hi);
        if resting {
            self.pos = self.snap(self.page);
        }
    }

    fn snap(&self, i: usize) -> f32 {
        self.snaps.get(i).copied().unwrap_or(0.)
    }

    fn t(&self, now: Instant) -> f32 {
        now.saturating_duration_since(self.epoch).as_secs_f32()
    }

    /// Allowed range of positions.
    fn bounds(&self) -> (f32, f32) {
        let (a, b) = (self.snap(self.lo), self.snap(self.hi));
        (a.min(b), a.max(b))
    }

    fn dimension(&self) -> f32 {
        let (a, b) = (self.snap(0), self.snap(self.snaps.len().saturating_sub(1)));
        ((a - b).abs() / (self.snaps.len().max(2) - 1) as f32).max(1.)
    }

    fn shape(&self, raw: f32) -> f32 {
        let (min, max) = self.bounds();
        let d = self.dimension();
        if raw > max {
            max + rubber_band(raw - max, d, BAND)
        } else if raw < min {
            min + rubber_band(raw - min, d, BAND)
        } else {
            raw
        }
    }

    fn unshape(&self, pos: f32) -> f32 {
        let (min, max) = self.bounds();
        let d = self.dimension();
        if pos > max {
            max + rubber_band_inverse(pos - max, d, BAND)
        } else if pos < min {
            min + rubber_band_inverse(pos - min, d, BAND)
        } else {
            pos
        }
    }

    /// Fingers down: take over from any running animation at the current position.
    pub fn begin(&mut self, now: Instant) {
        self.spring = None;
        self.dragging = true;
        self.from = self.page;
        self.raw = self.unshape(self.pos);
        self.tracker.reset();
        self.tracker.push(self.t(now), self.raw);
    }

    pub fn drag(&mut self, delta: f32, now: Instant) {
        if !self.dragging {
            self.begin(now);
        }
        self.raw += delta;
        self.pos = self.shape(self.raw);
        let t = self.t(now);
        self.tracker.push(t, self.raw);
    }

    /// Fingers up: pick the page by projected position and spring there with the release velocity.
    pub fn release(&mut self, now: Instant) -> usize {
        self.dragging = false;
        let v = self.tracker.velocity(self.t(now));
        let target = self.choose(project(self.pos, v, DECELERATION));
        self.go(target, v, now);
        target
    }

    /// The page a release lands on: the start page or one neighbor, toward the projection.
    pub fn choose(&self, projected: f32) -> usize {
        let start = self.snap(self.from);
        let toward = |i: usize| (self.snap(i) - start) * (projected - start) > 0.;
        let next = [self.from.checked_sub(1), Some(self.from + 1)]
            .into_iter()
            .flatten()
            .filter(|&i| i >= self.lo && i <= self.hi && i < self.snaps.len())
            .find(|&i| toward(i));
        match next {
            Some(n) if (projected - start).abs() >= self.commit * (self.snap(n) - start).abs() => n,
            _ => self.from,
        }
    }

    /// Animate to page `i` (keyboard, click), keeping any current velocity.
    pub fn animate_to(&mut self, i: usize, now: Instant) {
        let v = self.spring.map_or(0., |s| s.v);
        self.dragging = false;
        self.go(i.min(self.snaps.len().saturating_sub(1)), v, now);
    }

    fn go(&mut self, i: usize, v: f32, now: Instant) {
        self.page = i;
        self.last = now;
        if self.reduced {
            self.pos = self.snap(i);
            self.spring = None;
        } else {
            self.spring = Some(Spring::new(self.params, self.pos - self.snap(i), v));
        }
    }

    /// Put the pager on page `i` at rest, no animation.
    pub fn jump(&mut self, i: usize) {
        self.page = i;
        self.dragging = false;
        self.spring = None;
        self.pos = self.snap(i);
    }

    /// Advance the animation to `now`; returns the page once, when it has settled there.
    pub fn tick(&mut self, now: Instant) -> Option<usize> {
        let s = self.spring.as_mut()?;
        let dt = now.saturating_duration_since(self.last).as_secs_f32();
        self.last = now;
        s.step(dt);
        let x = s.x;
        if s.is_settled(0.5) {
            self.spring = None;
            self.pos = self.snap(self.page);
            return Some(self.page);
        }
        self.pos = self.snap(self.page) + x;
        None
    }

    pub fn position(&self) -> f32 {
        self.pos
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn moving(&self) -> bool {
        self.dragging || self.spring.is_some()
    }

    pub fn dragging(&self) -> bool {
        self.dragging
    }

    /// Fractional page under the current position (0 at the first snap, 1 at the second, ...).
    pub fn progress(&self) -> f32 {
        let n = self.snaps.len();
        if n < 2 {
            return 0.;
        }
        let (a, b) = (self.snap(0), self.snap(n - 1));
        if a == b {
            return 0.;
        }
        (self.pos - a) / (b - a) * (n - 1) as f32
    }
}

/// Left edge of each of `n` side-by-side pages `width` wide on a strip at `position` (page i rests at
/// `-i * width`): always exactly one width apart, so pages never overlap at any width.
pub fn page_offsets(position: f32, width: f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| position + i as f32 * width).collect()
}

#[cfg(test)]
#[path = "pager_tests.rs"]
mod tests;
