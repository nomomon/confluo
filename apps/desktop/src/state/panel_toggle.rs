//! Open/close transitions of the side panels toggled by keys, buttons or code (Cmd+B, Cmd+\, a click on
//! a row, ...). Every frame compares each panel's open state with the last frame's, so every toggle path
//! animates through one place; drags set the state through `set_panel_open`, which records it first.
//! Content swaps (another PR in the open preview) leave the state alone and never animate.

use std::time::Instant;

use super::Pane;
use crate::app::GitViewer;
use crate::ui::motion::reduced_motion;
use crate::ui::panel_anim::PanelAnim;

const PANES: [Pane; 4] = [Pane::Sidebar, Pane::Preview, Pane::Tree, Pane::Right];

impl Pane {
    /// Dev hook names: `tree`, `right`, `sidebar`, anything else is the preview.
    pub fn named(name: &str) -> Self {
        match name {
            "tree" => Pane::Tree,
            "right" => Pane::Right,
            "sidebar" => Pane::Sidebar,
            _ => Pane::Preview,
        }
    }
}

/// Whether a panel last seen `prev` (None: first frame) and `open` now starts a transition. The first
/// frame shows what is open as is; a drag animates itself; a content swap keeps `open` and never toggles.
fn toggled(prev: Option<bool>, open: bool, dragged: bool) -> bool {
    prev.is_some_and(|p| p != open) && !dragged
}

impl GitViewer {
    /// Record `p`'s current open state as seen (no transition for this change).
    pub(super) fn note_seen(&mut self, p: Pane) {
        let open = self.panel_open(p);
        let seen = &mut self.review.split.seen;
        match seen.iter_mut().find(|(q, _)| *q == p) {
            Some(s) => s.1 = open,
            None => seen.push((p, open)),
        }
    }

    /// Start or retarget a transition for every panel whose open state changed since the last frame.
    pub(super) fn detect_toggles(&mut self, now: Instant) {
        for p in PANES {
            let open = self.panel_open(p);
            let prev = self
                .review
                .split
                .seen
                .iter()
                .find(|(q, _)| *q == p)
                .map(|s| s.1);
            self.note_seen(p);
            let dragged = self.review.split.drag.is_some_and(|d| d.pane == p);
            let Some(prev) = prev.filter(|_| toggled(prev, open, dragged)) else {
                continue;
            };
            let rest = self.panel_rest_w(p);
            let drawn = self.panel_shown(p).unwrap_or(if prev { rest } else { 0. });
            let anims = &mut self.review.split.anims;
            let cur = anims
                .iter()
                .position(|(q, _)| *q == p)
                .map(|i| anims.remove(i).1);
            // A preview closed with its PR gone has nothing left to draw.
            if p == Pane::Preview && !open && self.detail.is_none() {
                continue;
            }
            let a = PanelAnim::toggled(cur, drawn, rest, open, reduced_motion(), now);
            self.review.split.anims.push((p, a));
        }
    }

    /// Width to draw this frame while a drag or spring moves the panel (also while collapsed).
    pub(crate) fn panel_shown(&self, p: Pane) -> Option<f32> {
        self.review
            .split
            .shown
            .iter()
            .find(|(q, _)| *q == p)
            .map(|(_, w)| *w)
    }

    /// Opacity of panel `p` this frame (below 1 only while a reduced-motion toggle fades it).
    pub(crate) fn panel_opacity(&self, p: Pane) -> f32 {
        self.review
            .split
            .opacity
            .iter()
            .find(|(q, _)| *q == p)
            .map_or(1., |(_, o)| *o)
    }

    /// `CONFLUO_DEV_PANEL=preview:0.4`: draws that panel 40% of the way through opening (opacity under
    /// reduced motion), frozen, for screenshots.
    pub(super) fn dev_panel_progress(
        &self,
        shown: &mut Vec<(Pane, f32)>,
        opacity: &mut Vec<(Pane, f32)>,
    ) {
        let Some((name, t)) = crate::dev::panel_progress() else {
            return;
        };
        let p = Pane::named(&name);
        let rest = self.panel_rest_w(p);
        if reduced_motion() {
            shown.push((p, rest));
            opacity.push((p, t));
        } else {
            shown.push((p, rest * t));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::toggled;

    #[test]
    fn only_real_toggles_animate() {
        assert!(toggled(Some(false), true, false));
        assert!(toggled(Some(true), false, false));
        // Another PR in the open preview: still open, content swap only.
        assert!(!toggled(Some(true), true, false));
        assert!(!toggled(None, true, false), "launch never animates");
        assert!(
            !toggled(Some(false), true, true),
            "drags animate themselves"
        );
    }
}
