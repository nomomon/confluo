//! Interactive two-finger page swipe (Safari-style back / forward), as a pure state machine fed by wheel
//! events. Only trackpad gestures with touch phases start it, only on the horizontal axis (the shared
//! per-gesture lock, so vertical scrolling never does), and only from events no horizontal block claimed.
//! The page follows the fingers 1:1 (with rubber-band resistance where there is nothing to go to);
//! release completes past a third of the width or on a fast flick, else springs back.

use std::time::Instant;

use gpui::{px, TouchPhase};

use crate::ui::motion::spring::{rubber_band, Driven, Spring};
use crate::ui::scroll::{gesture_axis, Axis};

/// Share of the width past which a release completes.
pub const COMMIT_AT: f32 = 0.33;
/// A flick this fast (px/s) toward the target completes from any distance past `FLICK_MIN`.
pub const FLICK_SPEED: f32 = 700.;
const FLICK_MIN: f32 = 0.04;
/// Stiffness of the settle (complete or cancel) spring.
const SETTLE_OMEGA: f32 = 24.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    /// Fingers move right: the previous page comes in from the left.
    Back,
    Forward,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// Not ours: let the event go on.
    Ignored,
    /// Tracking: redraw.
    Moved,
    /// Released: settle toward completing in `Some(dir)`, or back to rest.
    Released(Option<Dir>),
}

#[derive(Default)]
pub struct Swipe {
    lock: Option<Axis>,
    fingers: bool,
    tracking: bool,
    /// Finger travel along x since the swipe started (positive = back).
    tx: f32,
    /// Smoothed finger speed (px/s).
    vel: f32,
    last: Option<Instant>,
    settle: Option<(Driven, f32, Option<Dir>)>,
}

impl Swipe {
    /// Feed one wheel event. `width` is the page width, `can` says whether back / forward lead anywhere.
    #[allow(clippy::too_many_arguments)]
    pub fn wheel(&mut self, phase: TouchPhase, precise: bool, dx: f32, dy: f32, now: Instant, width: f32, can_back: bool, can_fwd: bool) -> Outcome {
        if !precise || self.settle.is_some() {
            return Outcome::Ignored;
        }
        let (axis, lock) = gesture_axis(self.lock, phase, px(dx), px(dy));
        self.lock = lock;
        match phase {
            TouchPhase::Started => {
                self.fingers = true;
                self.tracking = false;
                Outcome::Ignored
            }
            // Momentum after the fingers lifted never navigates.
            TouchPhase::Moved if !self.fingers => Outcome::Ignored,
            TouchPhase::Moved => {
                if !self.tracking {
                    if axis != Axis::X || dx == 0. {
                        return Outcome::Ignored;
                    }
                    self.tracking = true;
                    self.tx = 0.;
                    self.vel = 0.;
                    self.last = Some(now);
                }
                let dt = self.last.map(|l| now.saturating_duration_since(l).as_secs_f32()).unwrap_or(0.).max(1. / 240.);
                self.vel = self.vel * 0.6 + (dx / dt) * 0.4;
                self.last = Some(now);
                self.tx += dx;
                Outcome::Moved
            }
            TouchPhase::Ended => {
                self.fingers = false;
                if !std::mem::take(&mut self.tracking) {
                    return Outcome::Ignored;
                }
                // Fingers that rested before lifting carry no flick.
                if self.last.is_some_and(|l| now.saturating_duration_since(l).as_secs_f32() > 0.08) {
                    self.vel = 0.;
                }
                let dir = release(self.tx, self.vel, width, can_back, can_fwd);
                let from = self.offset_for(width, can_back, can_fwd);
                let target = match dir {
                    Some(Dir::Back) => width,
                    Some(Dir::Forward) => -width,
                    None => 0.,
                };
                let mut spring = Spring::new(from - target, self.vel.clamp(-4000., 4000.));
                spring.omega = SETTLE_OMEGA;
                self.settle = Some((Driven::new(spring), target, dir));
                self.tx = 0.;
                Outcome::Released(dir)
            }
        }
    }

    /// Shown page offset while tracking: 1:1 toward a page that exists, rubber band toward none.
    fn offset_for(&self, width: f32, can_back: bool, can_fwd: bool) -> f32 {
        let t = self.tx;
        let free = if t >= 0. { can_back } else { can_fwd };
        if !free {
            return rubber_band(t, width);
        }
        // Past a full page the fingers meet resistance too.
        let over = t.abs() - width;
        if over > 0. {
            return (width + rubber_band(over, width)).copysign(t);
        }
        t
    }

    /// The page offset to draw now (steps the settle spring). Returns the offset and, once a settle that
    /// completes has finished, the direction to navigate.
    pub fn frame(&mut self, now: Instant, width: f32, can_back: bool, can_fwd: bool) -> (f32, Option<Dir>) {
        if let Some((d, target, dir)) = self.settle.as_mut() {
            let x = d.advance(now) + *target;
            // Completing: done once the page is (almost) off; cancelling: once at rest.
            let done = d.spring.settled() || (dir.is_some() && (x - *target).abs() < 1.);
            if done {
                let (dir, target) = (*dir, *target);
                self.settle = None;
                return (if dir.is_some() { target } else { 0. }, dir);
            }
            return (x, None);
        }
        if self.tracking {
            return (self.offset_for(width, can_back, can_fwd), None);
        }
        (0., None)
    }

    /// Following the fingers or settling.
    pub fn active(&self) -> bool {
        self.tracking || self.settle.is_some()
    }

    /// Drop everything (the page changed under the gesture).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Where a release at travel `tx` with speed `vel` goes.
pub fn release(tx: f32, vel: f32, width: f32, can_back: bool, can_fwd: bool) -> Option<Dir> {
    let dir = if tx > 0. { Dir::Back } else { Dir::Forward };
    let allowed = match dir {
        Dir::Back => can_back,
        Dir::Forward => can_fwd,
    };
    if !allowed || width <= 0. || tx == 0. {
        return None;
    }
    let progress = tx.abs() / width;
    let toward = vel * tx.signum();
    (progress > COMMIT_AT || (toward > FLICK_SPEED && progress > FLICK_MIN)).then_some(dir)
}

#[cfg(test)]
#[path = "swipe_tests.rs"]
mod tests;
