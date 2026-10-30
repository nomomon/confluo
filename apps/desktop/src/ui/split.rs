//! Side panels that resize by dragging their divider and collapse when dragged past their minimum
//! (macOS split view / VS Code). Pure model, no gpui: the pointer gives a wanted width; below the minimum
//! the panel resists (rubber band) and past `COLLAPSE_AT` of the minimum a release collapses it; a
//! collapsed panel keeps a thin edge whose drag brings it back to its last width. Widths animate on a
//! spring (`motion::interactive::Animated`).

use std::time::Instant;

use crate::ui::motion::interactive::Animated;
use crate::ui::motion::physics::{rubber_band, BAND};

/// Dragging this share of the minimum below it collapses on release (40% under the minimum).
pub const COLLAPSE_AT: f32 = 0.4;
/// A collapsed panel opens once its edge is dragged this far out (px).
pub const OPEN_AT: f32 = 24.;
/// Grabbable edge left by a collapsed panel (px).
pub const EDGE_W: f32 = 5.;

/// Size rules of one panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub min: f32,
    pub max: f32,
}

/// Width to draw for a wanted width `want` while dragging: free in `min..=max`, resisting beyond.
pub fn drag_width(want: f32, l: Limits) -> f32 {
    if want < l.min {
        (l.min - rubber_band(l.min - want, l.min, BAND)).max(0.)
    } else if want > l.max {
        l.max + rubber_band(want - l.max, l.max.max(1.), BAND)
    } else {
        want
    }
}

/// What a released drag does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Release {
    /// Settle at this width (inside the limits).
    Width(f32),
    Collapse,
}

/// Release of a drag that wanted `want`: collapse when it went `COLLAPSE_AT` of the minimum under it.
pub fn release(want: f32, l: Limits) -> Release {
    if want < l.min * (1. - COLLAPSE_AT) {
        Release::Collapse
    } else {
        Release::Width(want.clamp(l.min, l.max))
    }
}

/// Release of a drag that started on a collapsed panel's edge: open when pulled out far enough.
pub fn release_collapsed(want: f32) -> bool {
    want >= OPEN_AT
}

/// One panel's animated width: a spring toward the open width or 0. `closing` marks a collapse in flight
/// (the caller sets its own open flag once it settles).
#[derive(Clone, Copy, Debug)]
pub struct PanelAnim {
    pub width: Animated,
    pub closing: bool,
}

impl PanelAnim {
    /// From `from` (drawn now) to `to`, starting the spring at rest.
    pub fn new(from: f32, to: f32, closing: bool, reduced: bool) -> Self {
        let now = Instant::now();
        let mut width = Animated::new(from);
        width.set(to, now, reduced);
        Self { width, closing }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: Limits = Limits {
        min: 200.,
        max: 400.,
    };

    #[test]
    fn drag_resists_outside_the_limits() {
        assert_eq!(drag_width(300., L), 300.);
        let a = drag_width(150., L);
        let b = drag_width(60., L);
        assert!(a < 200. && a > 150. && b < a && b > 60., "{a} {b}");
        assert!(drag_width(-500., L) >= 0.);
        assert!(drag_width(600., L) > 400. && drag_width(600., L) < 600.);
        // Fixed-width panels (min = max) only resist and collapse.
        let fixed = Limits {
            min: 240.,
            max: 240.,
        };
        assert_eq!(drag_width(240., fixed), 240.);
    }

    #[test]
    fn release_collapses_past_the_threshold() {
        assert_eq!(release(250., L), Release::Width(250.));
        assert_eq!(release(150., L), Release::Width(200.));
        assert_eq!(release(110., L), Release::Collapse);
        assert_eq!(release(900., L), Release::Width(400.));
        assert!(!release_collapsed(10.) && release_collapsed(30.));
    }

    #[test]
    fn collapse_animates_to_zero() {
        let mut p = PanelAnim::new(180., 0., true, false);
        let t = Instant::now();
        let mut last = 180.;
        for i in 1..100 {
            let (w, moving) = p.width.get(t + std::time::Duration::from_millis(i * 16));
            assert!(w <= last + 0.01);
            last = w;
            if !moving {
                break;
            }
        }
        assert_eq!(last, 0.);
        assert_eq!(
            PanelAnim::new(180., 0., true, true)
                .width
                .get(Instant::now()),
            (0., false)
        );
    }
}
