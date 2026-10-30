//! Motion tokens and entrance helpers. Features never pick durations themselves.
//!
//! Rules (see docs/design-system.md):
//! - Animate entrances of panels, dialogs, popovers and notices only.
//! - Never animate keyboard-driven repeated actions, tab switches, data
//!   refreshes or text changes.
//! - Hover and press use `.hover()` / `.active()` style swaps (GPUI has no
//!   style transitions); `HOVER` and `PRESS` document the intended feel.
//! - With reduced motion every duration is 0 and helpers render the element
//!   in its final state without wrapping it in an animation.

// The full token set is the public motion API even where unused today.
#![allow(dead_code)]

use gpui::{ease_in_out, ease_out_quint, linear, px, Animation, AnimationExt, AnyElement, ElementId, IntoElement, Pixels, Styled};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Upper bound for any duration.
pub const MAX_MS: u64 = 300;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Easing {
    Linear,
    EaseInOut,
    EaseOutQuint,
}

impl Easing {
    fn apply(self, t: f32) -> f32 {
        match self {
            Easing::Linear => linear(t),
            Easing::EaseInOut => ease_in_out(t),
            Easing::EaseOutQuint => ease_out_quint()(t),
        }
    }
}

/// A named duration plus easing curve.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Motion {
    pub ms: u64,
    pub easing: Easing,
}

pub const INSTANT: Motion = Motion { ms: 0, easing: Easing::Linear };
pub const HOVER: Motion = Motion { ms: 110, easing: Easing::EaseOutQuint };
pub const PRESS: Motion = Motion { ms: 70, easing: Easing::EaseOutQuint };
pub const FADE: Motion = Motion { ms: 150, easing: Easing::EaseInOut };
pub const PANEL: Motion = Motion { ms: 220, easing: Easing::EaseOutQuint };
/// Exits run at about 70% of the entry.
pub const PANEL_EXIT: Motion = Motion { ms: 154, easing: Easing::EaseOutQuint };
pub const LIST_INSERT: Motion = Motion { ms: 180, easing: Easing::EaseOutQuint };

static REDUCED: AtomicBool = AtomicBool::new(false);

/// Read the OS reduced-motion preference once at startup. Dev: `CONFLUO_REDUCE_MOTION=1` forces it
/// (screenshots of a window that gets no animation frames, e.g. while the screen is locked).
pub fn init() {
    REDUCED.store(std::env::var_os("CONFLUO_REDUCE_MOTION").is_some() || system_prefers_reduced_motion(), Ordering::Relaxed);
}

pub fn reduced_motion() -> bool {
    REDUCED.load(Ordering::Relaxed)
}

fn system_prefers_reduced_motion() -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }
    std::process::Command::new("defaults")
        .args(["read", "com.apple.universalaccess", "reduceMotion"])
        .output()
        .ok()
        .is_some_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
}

impl Motion {
    /// Effective duration: capped at `MAX_MS`, 0 under reduced motion.
    pub fn duration(self) -> Duration {
        if reduced_motion() {
            Duration::ZERO
        } else {
            Duration::from_millis(self.ms.min(MAX_MS))
        }
    }

    /// GPUI animation for this token, or `None` when it would be instant.
    pub fn animation(self) -> Option<Animation> {
        let d = self.duration();
        (!d.is_zero()).then(|| Animation::new(d).with_easing(move |t| self.easing.apply(t)))
    }
}

/// Run `animator` from 0 to 1 over `motion`, or render the final frame directly.
pub fn animate<E>(el: E, id: impl Into<ElementId>, motion: Motion, animator: impl Fn(E, f32) -> E + 'static) -> AnyElement
where
    E: IntoElement + 'static,
{
    match motion.animation() {
        Some(a) => el.with_animation(id, a, animator).into_any_element(),
        None => animator(el, 1.).into_any_element(),
    }
}

/// Fade from transparent to opaque (`FADE`). For notices, scrims, toasts.
pub fn fade_in<E: IntoElement + Styled + 'static>(el: E, id: impl Into<ElementId>) -> AnyElement {
    animate(el, id, FADE, |el, t| el.opacity(t))
}

/// Slide in from the right by `distance` while fading (`PANEL`). The element
/// is offset with `left`, so it must be relatively positioned.
pub fn slide_in_right<E: IntoElement + Styled + 'static>(el: E, id: impl Into<ElementId>, distance: Pixels) -> AnyElement {
    animate(el.relative(), id, PANEL, move |el, t| el.left(distance * (1. - t)).opacity(t))
}

/// Slide in from the left by `distance` while fading (`PANEL`), for a panel shown again (sidebar).
pub fn slide_in_left<E: IntoElement + Styled + 'static>(el: E, id: impl Into<ElementId>, distance: Pixels) -> AnyElement {
    animate(el.relative(), id, PANEL, move |el, t| el.left(-distance * (1. - t)).opacity(t))
}

/// Slide an element pinned to the right edge (`absolute().right_0()`) in from off-screen, plus fade.
/// Keeps its absolute position, unlike `slide_in_right`, which is for elements in the flow.
pub fn slide_in_from_edge<E: IntoElement + Styled + 'static>(el: E, id: impl Into<ElementId>, distance: Pixels) -> AnyElement {
    animate(el, id, PANEL, move |el, t| el.right(-distance * (1. - t)).opacity(t))
}

/// Small rise plus fade for dialogs and popovers (`FADE` timing, quint ease).
pub fn pop_in<E: IntoElement + Styled + 'static>(el: E, id: impl Into<ElementId>) -> AnyElement {
    let m = Motion { easing: Easing::EaseOutQuint, ..FADE };
    animate(el.relative(), id, m, |el, t| el.top(px(6.) * (1. - t)).opacity(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_capped_and_exit_is_shorter() {
        for m in [INSTANT, HOVER, PRESS, FADE, PANEL, PANEL_EXIT, LIST_INSERT] {
            assert!(m.ms <= MAX_MS);
        }
        const { assert!(PANEL_EXIT.ms * 10 == PANEL.ms * 7) };
        assert!(INSTANT.animation().is_none());
    }
}
