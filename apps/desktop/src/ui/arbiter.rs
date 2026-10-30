//! Gesture ownership. One trackpad gesture (fingers down to the next fingers down, so its momentum too)
//! has one axis and at most one owner, decided by its first movement and the handlers it reaches first
//! (innermost first: a block that can scroll that way, then the right panel's pager, then the page pager).
//! Every scroll handler asks the arbiter: while a pager or block owns a horizontal gesture, vertical
//! scrollers undo the wobble they received and nobody else reacts; a vertical gesture never reaches a pager.
//! Mouse wheels (line deltas) have no owner.

use std::cell::RefCell;

use gpui::{ScrollDelta, ScrollWheelEvent, TouchPhase};

use super::scroll::{dominant, Axis};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Owner {
    /// A vertical scroller (any): the gesture scrolls panes, never pages.
    Vertical,
    /// A horizontal block (table, code block, diff rows) by key.
    Block(String),
    /// The right panel's tab pager.
    Panel,
    /// The review back / forward pager.
    Page,
    /// The triage list's involvement tabs pager.
    ListTabs,
}

#[derive(Default, Debug)]
pub struct Arbiter {
    axis: Option<Axis>,
    owner: Option<Owner>,
    last: Option<(u8, u32, u32, u32, u32)>,
}

impl Arbiter {
    /// Note one event (repeat calls for the same event are ignored); returns the gesture's axis, if known.
    pub fn observe(
        &mut self,
        phase: TouchPhase,
        precise: bool,
        dx: f32,
        dy: f32,
        at: (f32, f32),
    ) -> Option<Axis> {
        if !precise {
            return None;
        }
        let key = (
            phase as u8,
            dx.to_bits(),
            dy.to_bits(),
            at.0.to_bits(),
            at.1.to_bits(),
        );
        if self.last == Some(key) {
            return self.axis;
        }
        self.last = Some(key);
        if matches!(phase, TouchPhase::Started) {
            self.axis = None;
            self.owner = None;
        }
        if self.axis.is_none() && (dx != 0. || dy != 0.) {
            self.axis = Some(dominant(gpui::px(dx), gpui::px(dy)));
        }
        self.axis
    }

    /// Take the gesture for `who` if nobody has it; true when `who` owns it.
    pub fn claim(&mut self, who: Owner) -> bool {
        match &self.owner {
            None => {
                self.owner = Some(who);
                true
            }
            Some(o) => *o == who,
        }
    }

    pub fn owner(&self) -> Option<&Owner> {
        self.owner.as_ref()
    }

    /// Some pager or block owns a horizontal gesture: vertical scrollers must not move.
    #[cfg(test)]
    pub fn horizontal_owned(&self) -> bool {
        self.axis == Some(Axis::X)
            && matches!(
                self.owner,
                Some(Owner::Block(_) | Owner::Panel | Owner::Page | Owner::ListTabs)
            )
    }
}

thread_local! {
    static ARB: RefCell<Arbiter> = RefCell::default();
}

fn parts(ev: &ScrollWheelEvent) -> (bool, f32, f32) {
    match ev.delta {
        ScrollDelta::Pixels(p) => (true, p.x.into(), p.y.into()),
        ScrollDelta::Lines(l) => (false, l.x, l.y),
    }
}

/// The window's arbiter: observe `ev`, returns its gesture axis (None for mouse wheels or no movement yet).
pub fn observe(ev: &ScrollWheelEvent) -> Option<Axis> {
    let (precise, dx, dy) = parts(ev);
    let at = (f32::from(ev.position.x), f32::from(ev.position.y));
    ARB.with(|a| a.borrow_mut().observe(ev.touch_phase, precise, dx, dy, at))
}

pub fn claim(who: Owner) -> bool {
    ARB.with(|a| a.borrow_mut().claim(who))
}

pub fn owner_is(who: &Owner) -> bool {
    ARB.with(|a| a.borrow().owner() == Some(who))
}

/// Whether horizontal block `key` handles `ev` along x. Trackpad: only if it owns the gesture, which it takes
/// at the gesture's first horizontal movement when it `can` still scroll that way (else the gesture goes on
/// to the panel or page and the block never moves during it). Mouse wheels: any x-dominant event.
pub fn block_handles_x(ev: &ScrollWheelEvent, key: &str, can: bool) -> bool {
    let (precise, dx, dy) = parts(ev);
    if !precise {
        return dominant(gpui::px(dx), gpui::px(dy)) == Axis::X;
    }
    if observe(ev) != Some(Axis::X) {
        return false;
    }
    let me = Owner::Block(key.to_string());
    ARB.with(|a| {
        let mut a = a.borrow_mut();
        if a.owner().is_none() && can {
            a.claim(me.clone());
        }
        a.owner() == Some(&me)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use TouchPhase::*;

    #[test]
    fn owner_by_start_target_no_chaining_momentum_stays() {
        let mut a = Arbiter::default();
        a.observe(Started, true, 0., 0., (1., 1.));
        assert_eq!(a.observe(Moved, true, -12., 2., (1., 1.)), Some(Axis::X));
        // Innermost handler first: the panel claims; the page pager after it gets nothing.
        assert!(a.claim(Owner::Panel));
        assert!(!a.claim(Owner::Page));
        // Wobble later in the gesture does not change the axis or the owner.
        assert_eq!(a.observe(Moved, true, 1., 9., (1., 2.)), Some(Axis::X));
        assert!(a.horizontal_owned());
        // Lift and momentum: still the panel's (no chaining to the page at the panel's edge).
        a.observe(Ended, true, 0., 0., (1., 2.));
        assert_eq!(a.observe(Moved, true, -30., 0., (1., 2.)), Some(Axis::X));
        assert!(!a.claim(Owner::Page) && a.claim(Owner::Panel));
        // The next gesture decides again.
        a.observe(Started, true, 0., 0., (5., 5.));
        assert_eq!(a.owner(), None);
    }

    #[test]
    fn list_gets_no_deltas_during_a_pager_gesture_and_vertical_never_reaches_a_pager() {
        let mut a = Arbiter::default();
        a.observe(Started, true, 0., 0., (0., 0.));
        a.observe(Moved, true, 20., 3., (0., 0.));
        assert!(a.claim(Owner::Page));
        // A vertical scroller seeing this gesture: horizontal and owned, it must undo its y.
        assert!(a.horizontal_owned());
        // Vertical gesture: the pane owns it; a pager claim fails.
        a.observe(Started, true, 0., 0., (0., 0.));
        assert_eq!(a.observe(Moved, true, 2., -15., (0., 0.)), Some(Axis::Y));
        assert!(a.claim(Owner::Vertical));
        assert!(!a.claim(Owner::Page) && !a.horizontal_owned());
        // The same event seen by several handlers counts once; mouse wheels have no axis.
        assert_eq!(a.observe(Moved, true, 2., -15., (0., 0.)), Some(Axis::Y));
        assert_eq!(a.observe(Moved, false, 40., 0., (0., 0.)), None);
    }
}
