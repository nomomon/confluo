//! Hold-Cmd shortcut hints: a small state machine, no rendering.
//!
//! Cmd pressed alone arms a timer (`timing::HINT_HOLD`); when it fires for the same press the hints
//! show. Any key press, releasing Cmd or the window losing focus hides them and cancels the timer,
//! so ordinary Cmd+key shortcuts never flash hints.

/// Hint visibility across one Cmd press.
#[derive(Default, Debug)]
pub struct HintHold {
    /// Bumped on every arm and cancel; a timer only shows hints for its own generation.
    gen: u64,
    /// Cmd is down (this press already armed or was spent by a key).
    held: bool,
    pending: bool,
    visible: bool,
    /// Dev: `CONFLUO_HINTS=1` keeps hints on.
    pub forced: bool,
}

impl HintHold {
    pub fn new(forced: bool) -> Self {
        Self { forced, ..Default::default() }
    }

    pub fn visible(&self) -> bool {
        self.visible || self.forced
    }

    /// Modifier state changed. Returns the generation to schedule a timer for, if one was armed.
    pub fn modifiers(&mut self, cmd: bool, others: bool) -> Option<u64> {
        if !cmd {
            self.reset();
            return None;
        }
        if !self.held {
            self.held = true;
            if !others {
                self.gen += 1;
                self.pending = true;
                return Some(self.gen);
            }
        } else if others && self.pending {
            // Chording (Cmd+Shift...) before the hints showed: not a hold.
            self.cancel();
        }
        None
    }

    /// A key was pressed: hide and stay hidden until Cmd is released.
    pub fn key_down(&mut self) -> bool {
        let was = self.visible || self.pending;
        self.cancel();
        was
    }

    /// The window lost focus.
    pub fn blur(&mut self) -> bool {
        let was = self.visible || self.pending;
        self.reset();
        was
    }

    /// The hold timer for `gen` elapsed. Returns true when the hints became visible.
    pub fn fire(&mut self, gen: u64) -> bool {
        if self.pending && gen == self.gen {
            self.pending = false;
            self.visible = true;
            return true;
        }
        false
    }

    fn cancel(&mut self) {
        self.gen += 1;
        self.pending = false;
        self.visible = false;
    }

    fn reset(&mut self) {
        self.cancel();
        self.held = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_shows_after_timer() {
        let mut h = HintHold::default();
        let g = h.modifiers(true, false).unwrap();
        assert!(!h.visible());
        assert!(h.fire(g));
        assert!(h.visible());
        // Adding Shift once visible keeps them (for the Cmd+Shift hints).
        assert_eq!(h.modifiers(true, true), None);
        assert!(h.visible());
        h.modifiers(false, false);
        assert!(!h.visible());
    }

    #[test]
    fn key_during_hold_cancels() {
        let mut h = HintHold::default();
        let g = h.modifiers(true, false).unwrap();
        assert!(h.key_down());
        assert!(!h.fire(g) && !h.visible());
        // Still holding Cmd after Cmd+R: no re-arm until released.
        assert_eq!(h.modifiers(true, false), None);
        h.modifiers(false, false);
        assert!(h.modifiers(true, false).is_some());
    }

    #[test]
    fn release_and_blur_cancel_and_hide() {
        let mut h = HintHold::default();
        let g = h.modifiers(true, false).unwrap();
        h.modifiers(false, false);
        assert!(!h.fire(g));
        let g = h.modifiers(true, false).unwrap();
        assert!(h.fire(g));
        assert!(h.blur() && !h.visible());
        // Stale timer of an older press does nothing.
        let old = h.modifiers(true, false).unwrap();
        h.modifiers(false, false);
        let _new = h.modifiers(true, false).unwrap();
        assert!(!h.fire(old));
    }

    #[test]
    fn chord_before_timer_cancels_and_forced_stays() {
        let mut h = HintHold::default();
        let g = h.modifiers(true, false).unwrap();
        h.modifiers(true, true);
        assert!(!h.fire(g));
        assert_eq!(HintHold::default().modifiers(true, true), None);
        let f = HintHold::new(true);
        assert!(f.visible());
    }
}
