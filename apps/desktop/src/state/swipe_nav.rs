//! Two-finger swipe navigation (Review mode back / forward) and swipes between the right panel tabs,
//! driven by `ui::swipe::Swipe`. The window root listens for wheel events nothing claimed; the right
//! panel listens first and takes a sideways swipe only when it can switch tabs that way.

use std::time::Instant;

use gpui::{px, Context, ScrollDelta, ScrollWheelEvent, Window};

use crate::app::{GitViewer, Overlay};
use crate::ui::motion::spring::{Driven, Spring};
use crate::ui::motion::reduced_motion;
use crate::ui::scroll::{claims_x, locked_axis, Axis};
use crate::ui::swipe::{Dir, Outcome, Swipe};

use super::RightTab;

#[derive(Default)]
pub struct SwipeUi {
    pub page: Swipe,
    pub panel: Swipe,
    /// Page and panel offsets for this frame (set at the start of the render).
    pub page_x: f32,
    pub panel_x: f32,
    /// The tab a panel swipe switched to slides in from this offset.
    panel_enter: Option<Driven>,
    /// Reduced motion: the view a swipe switched to fades in (bumped per switch, the fade's id).
    pub fade: Option<usize>,
}

fn wheel_parts(ev: &ScrollWheelEvent) -> (bool, f32, f32) {
    match ev.delta {
        ScrollDelta::Pixels(p) => (true, p.x.into(), p.y.into()),
        ScrollDelta::Lines(l) => (false, l.x, l.y),
    }
}

impl GitViewer {
    fn swipe_allowed(&self) -> bool {
        self.settings.swipe_pages && matches!(self.overlay, Overlay::None) && !self.layer_open() && !self.review.quick_open
    }

    /// Back always leads somewhere in Review mode (the list at least); forward only with forward history.
    fn page_targets(&self) -> (bool, bool) {
        (self.reviewing, self.reviewing && self.nav.pos + 1 < self.nav.history.len())
    }

    /// Window root: wheel events no block or panel claimed.
    pub(crate) fn page_wheel(&mut self, ev: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !(self.reviewing || self.review.swipe.page.active()) || !self.swipe_allowed() {
            return;
        }
        let (precise, dx, dy) = wheel_parts(ev);
        let (b, f) = self.page_targets();
        let w = f32::from(window.viewport_size().width);
        match self.review.swipe.page.wheel(ev.touch_phase, precise, dx, dy, Instant::now(), w, b, f) {
            Outcome::Ignored => {}
            Outcome::Moved => cx.notify(),
            Outcome::Released(dir) => {
                if reduced_motion() {
                    // No tracking under reduced motion: switch at once with a short fade.
                    self.review.swipe.page.reset();
                    if let Some(d) = dir {
                        self.swipe_go(d, cx);
                    }
                }
                cx.notify()
            }
        }
    }

    fn swipe_go(&mut self, d: Dir, cx: &mut Context<Self>) {
        self.review.swipe.page.reset();
        if reduced_motion() {
            self.review.swipe.fade = Some(self.review.swipe.fade.map_or(1, |n| n + 1));
        }
        self.history_step(if d == Dir::Back { -1 } else { 1 }, cx);
        cx.notify();
    }

    /// Right panel: a sideways swipe switches Overview / Conversation when there is a tab that way.
    pub(crate) fn panel_wheel(&mut self, ev: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.swipe_allowed() {
            return;
        }
        let (precise, dx, dy) = wheel_parts(ev);
        let (b, f) = (self.review.right_tab == RightTab::Conversation, self.review.right_tab == RightTab::Overview);
        if locked_axis("right-panel", ev.touch_phase, px(dx), px(dy)) == Axis::X {
            let can = if dx > 0. { b } else { dx < 0. && f };
            if !claims_x("right-panel", ev.touch_phase, precise, can) {
                return;
            }
            cx.stop_propagation();
        }
        let w = f32::from(window.viewport_size().width) * self.review.right_ratio;
        let now = Instant::now();
        match self.review.swipe.panel.wheel(ev.touch_phase, precise, dx, dy, now, w, b, f) {
            Outcome::Ignored => {}
            Outcome::Moved | Outcome::Released(None) => cx.notify(),
            Outcome::Released(Some(d)) => {
                let x = self.review.swipe.panel_x;
                self.review.swipe.panel.reset();
                self.review.right_tab = if d == Dir::Back { RightTab::Overview } else { RightTab::Conversation };
                // The new tab continues the fingers' motion and settles in place.
                let from = if d == Dir::Back { x - w } else { x + w };
                self.review.swipe.panel_enter = (!reduced_motion()).then(|| Driven::new(Spring { omega: 24., ..Spring::new(from, 0.) }));
                cx.notify()
            }
        }
    }

    /// Step both swipes for this frame (start of the window render); completes a finished page swipe.
    pub(crate) fn swipe_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        let w = f32::from(window.viewport_size().width);
        let (b, f) = self.page_targets();
        let s = &mut self.review.swipe;
        let (x, done) = s.page.frame(now, w, b, f);
        s.page_x = if reduced_motion() { 0. } else { x };
        let pw = w * self.review.right_ratio;
        let (px_, _) = s.panel.frame(now, pw, self.review.right_tab == RightTab::Conversation, self.review.right_tab == RightTab::Overview);
        let enter = s.panel_enter.as_mut().map(|d| (d.advance(now), d.spring.settled()));
        s.panel_x = if reduced_motion() { 0. } else { px_ + enter.map_or(0., |e| e.0) };
        if enter.is_some_and(|e| e.1) {
            s.panel_enter = None;
        }
        let animating = s.page.active() || s.panel.active() || s.panel_enter.is_some();
        if let Some(d) = done {
            cx.defer_in(window, move |this, _, cx| this.swipe_go(d, cx));
        }
        if let Some(p) = crate::dev::swipe() {
            self.review.swipe.page_x = if self.reviewing { p * w } else { 0. };
        }
        if animating {
            window.request_animation_frame();
        }
    }
}
