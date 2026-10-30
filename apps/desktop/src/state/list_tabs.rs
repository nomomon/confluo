//! Swiping between the triage list's involvement tabs (Authored, Reviewing, All) on a `Pager`: a sideways
//! gesture that starts on the list (arbiter owner `ListTabs`) slides the tab lists side by side; release
//! snaps to the projected tab. Clicks and 1/2/3 move through the same spring. Each tab keeps its own
//! scroll position (one scroll handle per tab).

use std::time::Instant;

use gpui::{Context, ScrollDelta, ScrollHandle, ScrollWheelEvent, Window};
use gv_engine::{ChangeRequest, Involvement};

use crate::app::GitViewer;
use crate::ui::arbiter::{self, Owner};
use crate::ui::motion::interactive::{Drag, Pager, SwipeGate};
use crate::ui::motion::reduced_motion;
use crate::ui::scroll::Axis;

pub const INVOLVEMENTS: [(&str, Involvement); 3] = [
    ("Authored", Involvement::Authored),
    ("Reviewing", Involvement::Reviewing),
    ("All", Involvement::All),
];

pub struct ListTabsUi {
    gate: SwipeGate,
    pub pager: Pager,
    /// Scroll handles of the tabs not on screen (the current one is `list_scroll`).
    pub scrolls: [ScrollHandle; 3],
    /// Items of the neighbor tabs, loaded when a swipe begins.
    pub cache: [Option<Vec<ChangeRequest>>; 3],
    /// Strip position this frame (page i rests at -i * width) and the page width.
    pub x: f32,
    pub width: f32,
    pub progress: f32,
    /// Width of the list area as last laid out (written while painting).
    pub measured: std::rc::Rc<std::cell::Cell<f32>>,
}

impl Default for ListTabsUi {
    fn default() -> Self {
        Self {
            gate: SwipeGate::default(),
            pager: Pager::new(vec![0., -1., -2.], 2),
            scrolls: Default::default(),
            cache: Default::default(),
            x: 0.,
            width: 1.,
            progress: 2.,
            measured: Default::default(),
        }
    }
}

pub fn tab_index(t: Involvement) -> usize {
    INVOLVEMENTS.iter().position(|(_, i)| *i == t).unwrap_or(2)
}

/// Pages to draw: the current one, plus its neighbors while the strip moves (never more than needed).
pub fn pages_to_draw(current: usize, progress: f32, moving: bool) -> Vec<usize> {
    if !moving {
        return vec![current];
    }
    let (lo, hi) = (
        progress.floor().max(0.) as usize,
        (progress.ceil().max(0.) as usize).min(2),
    );
    let mut v: Vec<usize> = (lo..=hi).collect();
    if !v.contains(&current) {
        v.push(current);
    }
    v
}

impl GitViewer {
    /// Show tab `i`: swap in its scroll position and reload; `animate` slides the strip there.
    pub(crate) fn switch_list_tab(&mut self, i: usize, animate: bool, cx: &mut Context<Self>) {
        let cur = tab_index(self.tab);
        if i != cur {
            let t = &mut self.review.swipe.tabs;
            let next = std::mem::take(&mut t.scrolls[i]);
            t.scrolls[cur] = std::mem::replace(&mut self.list_scroll, next);
            self.tab = INVOLVEMENTS[i].1;
            self.reload();
        }
        let p = &mut self.review.swipe.tabs.pager;
        p.reduced = reduced_motion();
        if animate {
            p.animate_to(i, Instant::now());
        }
        cx.notify();
    }

    fn fill_tab_cache(&mut self) {
        let cur = tab_index(self.tab);
        for (i, (_, inv)) in INVOLVEMENTS.iter().enumerate() {
            if i.abs_diff(cur) == 1 {
                let items = self.listing_for(*inv);
                self.review.swipe.tabs.cache[i] = Some(items);
            }
        }
    }

    /// Wheel events on the list: a sideways trackpad gesture pages between the tabs.
    pub(crate) fn list_tabs_wheel(
        &mut self,
        ev: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.swipe_pages || self.reviewing {
            return;
        }
        if arbiter::observe(ev) == Some(Axis::X) && !arbiter::claim(Owner::ListTabs) {
            return;
        }
        if arbiter::owner_is(&Owner::ListTabs) {
            cx.stop_propagation();
        }
        let (precise, dx, dy) = match ev.delta {
            ScrollDelta::Pixels(p) => (true, f32::from(p.x), f32::from(p.y)),
            ScrollDelta::Lines(l) => (false, l.x, l.y),
        };
        let drag = self
            .review
            .swipe
            .tabs
            .gate
            .feed(ev.touch_phase, precise, dx, dy);
        let now = Instant::now();
        let p = &mut self.review.swipe.tabs.pager;
        p.reduced = reduced_motion();
        match drag {
            Drag::None => return,
            Drag::Begin(dx) => {
                p.begin(now);
                p.drag(dx, now);
                self.fill_tab_cache();
            }
            Drag::Move(dx) => p.drag(dx, now),
            Drag::Release => {
                let t = p.release(now);
                self.switch_list_tab(t, false, cx);
            }
        }
        cx.notify()
    }

    /// Start of the window render: step the strip; clicks, keys and other code that set the tab slide it.
    pub(crate) fn list_tabs_frame(&mut self, window: &mut Window) {
        let w = self.review.swipe.tabs.measured.get().max(1.);
        let cur = tab_index(self.tab);
        let t = &mut self.review.swipe.tabs;
        t.pager.set_snaps(vec![0., -w, -2. * w], 0, 2);
        if !t.pager.dragging() && t.pager.page() != cur {
            t.pager.animate_to(cur, Instant::now());
        }
        t.pager.tick(Instant::now());
        t.width = w;
        t.x = t.pager.position();
        t.progress = t.pager.progress();
        if let Some(p) = crate::dev::list_tabs() {
            t.x = -p * w;
            t.progress = p;
        }
        if t.pager.moving() {
            window.request_animation_frame();
        } else {
            t.cache = Default::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::motion::interactive::page_offsets;

    #[test]
    fn tab_pages_tile_and_draw_only_neighbors() {
        for w in [420., 900.] {
            // On Reviewing (page 1) the strip sits at -w: Authored left, All right, adjacent.
            assert_eq!(page_offsets(-w, w, 3), vec![-w, 0., w]);
        }
        assert_eq!(pages_to_draw(1, 1., false), vec![1]);
        assert_eq!(pages_to_draw(1, 1.4, true), vec![1, 2]);
        assert_eq!(pages_to_draw(2, 1.6, true), vec![1, 2]);
        assert_eq!(pages_to_draw(0, -0.2, true), vec![0]);
    }

    #[test]
    fn vertical_gestures_never_page() {
        let mut g = SwipeGate::default();
        g.feed(gpui::TouchPhase::Started, true, 0., 0.);
        assert_eq!(g.feed(gpui::TouchPhase::Moved, true, 2., -14.), Drag::None);
        assert_eq!(g.feed(gpui::TouchPhase::Moved, true, 30., -1.), Drag::None);
        assert_eq!(g.feed(gpui::TouchPhase::Ended, true, 0., 0.), Drag::None);
    }
}
