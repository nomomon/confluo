//! Two-finger swipes on `motion::interactive::Pager`: Review mode back / forward (pages forward, current,
//! back at -w, 0, +w; a third of the width or a flick commits) and the right panel tabs (Overview at 0,
//! Conversation at -w). Every tab switch, by swipe, click or key, moves through the same spring.
//! The window root gets wheel events nothing claimed; the right panel listens first and takes a sideways
//! swipe only when it has a tab that way.

use std::time::Instant;

use gpui::{Context, ScrollDelta, ScrollWheelEvent, Window};

use crate::app::{GitViewer, Overlay};
use crate::ui::arbiter::{self, Owner};
use crate::ui::motion::interactive::{Drag, Pager, SwipeGate};
use crate::ui::motion::reduced_motion;
use crate::ui::scroll::Axis;

use super::RightTab;

/// Page index of the review (forward is 0, back is 2).
const HERE: usize = 1;

pub struct SwipeUi {
    page_gate: SwipeGate,
    pub page: Pager,
    panel_gate: SwipeGate,
    pub panel: Pager,
    /// Offsets for this frame (set at the start of the render): review translate, panel strip translate
    /// (0 shows Overview, -width Conversation).
    pub page_x: f32,
    pub panel_x: f32,
    /// Fractional tab under the panel strip (0 Overview, 1 Conversation).
    pub panel_progress: f32,
    /// What a page slide that is leaving does when it lands (Esc / back button / Cmd+[ / swipe).
    out: Option<Out>,
    /// Reduced motion: the view a swipe switched to fades in (bumped per switch, the fade's id).
    pub fade: Option<usize>,
}

impl Default for SwipeUi {
    fn default() -> Self {
        let mut page = Pager::new(vec![-1., 0., 1.], HERE);
        // Like Safari: a third of the way commits a slow swipe back.
        page.commit = 0.33;
        Self {
            out: None,
            page_gate: SwipeGate::default(),
            page,
            panel_gate: SwipeGate::default(),
            panel: Pager::new(vec![0., -1.], 0),
            page_x: 0.,
            panel_x: 0.,
            panel_progress: 0.,
            fade: None,
        }
    }
}

/// Where a review sliding out goes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Out {
    /// `exit_review` (`navigating`: the history already moved).
    Exit(bool),
}

fn wheel_parts(ev: &ScrollWheelEvent) -> (bool, f32, f32) {
    match ev.delta {
        ScrollDelta::Pixels(p) => (true, p.x.into(), p.y.into()),
        ScrollDelta::Lines(l) => (false, l.x, l.y),
    }
}

fn tab_index(t: RightTab) -> usize {
    if t == RightTab::Overview {
        0
    } else {
        1
    }
}

/// Feed one gate result to a pager; returns the page a release targets.
fn drive(p: &mut Pager, drag: Drag, now: Instant) -> Option<usize> {
    p.reduced = reduced_motion();
    match drag {
        Drag::None => None,
        Drag::Begin(dx) => {
            p.begin(now);
            p.drag(dx, now);
            None
        }
        Drag::Move(dx) => {
            p.drag(dx, now);
            None
        }
        Drag::Release => Some(p.release(now)),
    }
}

impl GitViewer {
    fn swipe_allowed(&self) -> bool {
        self.settings.swipe_pages
            && matches!(self.overlay, Overlay::None)
            && !self.layer_open()
            && !self.review.quick_open
    }

    /// Window root: gestures nothing inside claimed (`ui::arbiter`): the review pager takes a sideways one;
    /// once it owns a gesture every event of it (both axes, momentum) is its alone.
    pub(crate) fn page_wheel(
        &mut self,
        ev: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.reviewing || !self.swipe_allowed() {
            return;
        }
        if arbiter::observe(ev) == Some(Axis::X) && !arbiter::claim(Owner::Page) {
            return;
        }
        let (precise, dx, dy) = wheel_parts(ev);
        let drag = self
            .review
            .swipe
            .page_gate
            .feed(ev.touch_phase, precise, dx, dy);
        if drag == Drag::None {
            return;
        }
        let target = drive(&mut self.review.swipe.page, drag, Instant::now());
        if reduced_motion() {
            // No tracking under reduced motion: switch on release with a short fade.
            if let Some(t) = target.filter(|t| *t != HERE) {
                self.swipe_go(t, cx);
            }
        }
        cx.notify()
    }

    fn swipe_go(&mut self, page: usize, cx: &mut Context<Self>) {
        self.review.swipe.page.jump(HERE);
        self.review.swipe.page_x = 0.;
        if reduced_motion() {
            self.bump_fade();
        }
        match self.review.swipe.out.take() {
            Some(Out::Exit(nav)) => {
                self.navigating = nav;
                self.exit_review(cx);
                self.navigating = false;
            }
            None => self.history_step_now(if page > HERE { -1 } else { 1 }, cx),
        }
        cx.notify();
    }

    fn bump_fade(&mut self) {
        self.review.swipe.fade = Some(self.review.swipe.fade.map_or(1, |n| n + 1));
    }

    /// Leave Review mode by sliding the review out to the right over the list (Esc, back button, Cmd+[ to
    /// the list). `navigating`: the history already moved. Reduced motion: switch with a short fade.
    pub(crate) fn leave_review(&mut self, navigating: bool, cx: &mut Context<Self>) {
        if !self.reviewing {
            return;
        }
        if reduced_motion() {
            self.bump_fade();
            self.navigating = navigating;
            self.exit_review(cx);
            self.navigating = false;
            return;
        }
        self.review.swipe.out = Some(Out::Exit(navigating));
        self.review.swipe.page.animate_to(2, Instant::now());
        cx.notify();
    }

    /// Entering Review mode from the list: the review slides in from the right (reverse of back), over
    /// the list at -30% and dimmed; a swipe can grab it mid-way. Reduced motion: a short fade.
    pub(crate) fn slide_review_in(&mut self) {
        if reduced_motion() {
            self.bump_fade();
            return;
        }
        let s = &mut self.review.swipe;
        s.out = None;
        s.page.jump(2);
        s.page.animate_to(HERE, Instant::now());
    }

    /// Right panel: a sideways gesture that starts in the panel belongs to its pager (unless a block inside
    /// took it): it moves between Overview and Conversation and rubber-bands at the ends, never chaining
    /// into the page swipe.
    pub(crate) fn panel_wheel(
        &mut self,
        ev: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.swipe_allowed() {
            return;
        }
        if arbiter::observe(ev) == Some(Axis::X) && !arbiter::claim(Owner::Panel) {
            return;
        }
        if arbiter::owner_is(&Owner::Panel) {
            cx.stop_propagation();
        }
        let (precise, dx, dy) = wheel_parts(ev);
        let drag = self
            .review
            .swipe
            .panel_gate
            .feed(ev.touch_phase, precise, dx, dy);
        if drag == Drag::None {
            return;
        }
        if let Some(t) = drive(&mut self.review.swipe.panel, drag, Instant::now()) {
            self.review.right_tab = if t == 0 {
                RightTab::Overview
            } else {
                RightTab::Conversation
            };
        }
        cx.notify()
    }

    /// Step both pagers for this frame (start of the window render); completes a finished page swipe.
    pub(crate) fn swipe_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        let w = f32::from(window.viewport_size().width);
        let can_fwd = self.nav.pos + 1 < self.nav.history.len();
        let tab = tab_index(self.review.right_tab);
        let pw = (w * self.review.right_ratio).max(1.);
        let s = &mut self.review.swipe;
        s.page
            .set_snaps(vec![-w, 0., w], if can_fwd { 0 } else { HERE }, 2);
        let settled = s.page.tick(now);
        if settled == Some(HERE) {
            // Grabbed and brought back: the pending exit is off.
            s.out = None;
        }
        let done = settled.filter(|p| *p != HERE);
        // Nothing ahead: a leftward drag shows nothing (Safari), the review stays put.
        s.page_x = if reduced_motion() {
            0.
        } else if can_fwd {
            s.page.position()
        } else {
            s.page.position().max(0.)
        };
        s.panel.set_snaps(vec![0., -pw], 0, 1);
        s.panel.reduced = reduced_motion();
        // Clicks, keys and other code set the tab; the strip follows through the same spring.
        if !s.panel.dragging() && s.panel.page() != tab {
            s.panel.animate_to(tab, now);
        }
        s.panel.tick(now);
        s.panel_x = s.panel.position();
        s.panel_progress = s.panel.progress();
        let animating = s.page.moving() || s.panel.moving();
        if let Some(p) = done {
            cx.defer_in(window, move |this, _, cx| this.swipe_go(p, cx));
        }
        if let Some(p) = crate::dev::swipe() {
            self.review.swipe.page_x = if self.reviewing { p * w } else { 0. };
        }
        // `CONFLUO_DEV_ENTER=0.3`: Review mode 30% into its slide-in.
        if let Some(p) = crate::dev::enter() {
            self.review.swipe.page_x = if self.reviewing { (1. - p) * w } else { 0. };
        }
        if animating {
            window.request_animation_frame();
        }
    }
}
