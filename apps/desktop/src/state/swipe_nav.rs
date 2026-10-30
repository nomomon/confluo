//! Two-finger swipes on `motion::interactive::Pager`: Review mode back / forward (pages forward, current,
//! back at -w, 0, +w; a third of the width or a flick commits) and the right panel tabs (Overview at 0,
//! Conversation at -w). Every tab switch, by swipe, click or key, moves through the same spring.
//! The window root gets wheel events nothing claimed; the right panel listens first and takes a sideways
//! swipe only when it has a tab that way.

use std::time::Instant;

use gpui::{px, Context, ScrollDelta, ScrollWheelEvent, Window};

use crate::app::{GitViewer, Overlay};
use crate::ui::motion::interactive::{Drag, Pager, SwipeGate};
use crate::ui::motion::reduced_motion;
use crate::ui::scroll::{claims_x, locked_axis, Axis};

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
    /// Reduced motion: the view a swipe switched to fades in (bumped per switch, the fade's id).
    pub fade: Option<usize>,
}

impl Default for SwipeUi {
    fn default() -> Self {
        let mut page = Pager::new(vec![-1., 0., 1.], HERE);
        // Like Safari: a third of the way commits a slow swipe back.
        page.commit = 0.33;
        Self { page_gate: SwipeGate::default(), page, panel_gate: SwipeGate::default(), panel: Pager::new(vec![0., -1.], 0), page_x: 0., panel_x: 0., panel_progress: 0., fade: None }
    }
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
        self.settings.swipe_pages && matches!(self.overlay, Overlay::None) && !self.layer_open() && !self.review.quick_open
    }

    /// Window root: wheel events no block or panel claimed.
    pub(crate) fn page_wheel(&mut self, ev: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.reviewing || !self.swipe_allowed() {
            return;
        }
        let (precise, dx, dy) = wheel_parts(ev);
        let drag = self.review.swipe.page_gate.feed(ev.touch_phase, precise, dx, dy);
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
            self.review.swipe.fade = Some(self.review.swipe.fade.map_or(1, |n| n + 1));
        }
        self.history_step(if page > HERE { -1 } else { 1 }, cx);
        cx.notify();
    }

    /// Right panel: a sideways swipe moves between Overview and Conversation when there is a tab that way.
    pub(crate) fn panel_wheel(&mut self, ev: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.swipe_allowed() {
            return;
        }
        let (precise, dx, dy) = wheel_parts(ev);
        let at = self.review.swipe.panel.page();
        if locked_axis("right-panel", ev.touch_phase, px(dx), px(dy)) == Axis::X && !self.review.swipe.panel_gate.tracking() {
            let can = if dx > 0. { at == 1 } else { dx < 0. && at == 0 };
            if !claims_x("right-panel", ev.touch_phase, precise, can) {
                return;
            }
        }
        let drag = self.review.swipe.panel_gate.feed(ev.touch_phase, precise, dx, dy);
        if drag == Drag::None {
            return;
        }
        cx.stop_propagation();
        if let Some(t) = drive(&mut self.review.swipe.panel, drag, Instant::now()) {
            self.review.right_tab = if t == 0 { RightTab::Overview } else { RightTab::Conversation };
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
        s.page.set_snaps(vec![-w, 0., w], if can_fwd { 0 } else { HERE }, 2);
        let done = s.page.tick(now).filter(|p| *p != HERE);
        s.page_x = if reduced_motion() { 0. } else { s.page.position() };
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
        if animating {
            window.request_animation_frame();
        }
    }
}
