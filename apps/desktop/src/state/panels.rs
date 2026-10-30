//! Drag-to-resize and drag-to-collapse for every side panel (sidebar, triage preview, review file tree,
//! review right panel) on the shared `ui::split` model. One drag at a time; widths spring on release.

use std::time::Instant;

use gpui::{Context, Pixels, Point};

use super::Pane;
use crate::app::GitViewer;
use crate::features::layout;
use crate::ui::motion::reduced_motion;
use crate::ui::panel_anim::PanelAnim;
use crate::ui::split::{drag_width, release, release_collapsed, Limits, Release};

/// A divider (or a collapsed panel's edge) being dragged.
#[derive(Clone, Copy, Debug)]
pub struct SplitDrag {
    pub pane: Pane,
    /// Started on a collapsed panel's edge.
    pub collapsed: bool,
    /// Width the pointer asks for.
    pub want: f32,
}

#[derive(Default)]
pub struct SplitUi {
    pub drag: Option<SplitDrag>,
    pub(super) anims: Vec<(Pane, PanelAnim)>,
    /// Widths to draw this frame where they differ from the resting layout (set by `split_frame`).
    pub(super) shown: Vec<(Pane, f32)>,
    /// Opacity while a reduced-motion toggle fades the panel.
    pub(super) opacity: Vec<(Pane, f32)>,
    /// Open state of each panel at the last frame (`panel_toggle`).
    pub(super) seen: Vec<(Pane, bool)>,
}

impl GitViewer {
    pub(crate) fn panel_open(&self, p: Pane) -> bool {
        match p {
            Pane::Tree => self.review.tree_open,
            Pane::Right => self.review.right_open,
            Pane::Sidebar => !self.nav.sidebar_hidden,
            Pane::Preview => !self.preview_hidden && self.detail.is_some(),
        }
    }

    fn set_panel_open(&mut self, p: Pane, open: bool, cx: &mut Context<Self>) {
        if self.panel_open(p) == open {
            return;
        }
        match p {
            Pane::Tree => self.review.tree_open = open,
            Pane::Right => self.review.right_open = open,
            Pane::Sidebar => self.nav.sidebar_hidden = !open,
            Pane::Preview => self.toggle_detail(cx),
        }
        self.note_seen(p);
    }

    pub(crate) fn panel_limits(&self, p: Pane) -> Limits {
        match p {
            Pane::Tree => Limits {
                min: layout::tree_min_w().into(),
                max: layout::tree_max_w().into(),
            },
            Pane::Right => Limits {
                min: layout::right_min_w().into(),
                max: f32::from(self.review_content_w() - layout::diff_min_w())
                    .max(layout::right_min_w().into()),
            },
            Pane::Sidebar => Limits {
                min: self.sidebar_w_open().into(),
                max: self.sidebar_w_open().into(),
            },
            Pane::Preview => Limits {
                min: layout::preview_w().into(),
                max: layout::preview_w().into(),
            },
        }
    }

    /// Width of the panel at rest (its last width).
    pub(crate) fn panel_rest_w(&self, p: Pane) -> f32 {
        match p {
            Pane::Tree => self
                .tree_w()
                .min(
                    (self.viewport_w - layout::diff_min_w() - layout::right_min_w())
                        .max(layout::tree_min_w()),
                )
                .into(),
            Pane::Right => {
                layout::right_pane_w(self.review_content_w(), self.review.right_ratio).into()
            }
            Pane::Sidebar => self.sidebar_w_open().into(),
            Pane::Preview => layout::preview_w().into(),
        }
    }

    /// Pointer down on a divider or a collapsed panel's edge.
    pub(crate) fn start_panel_drag(&mut self, p: Pane, cx: &mut Context<Self>) {
        let collapsed = !self.panel_open(p);
        let want = if collapsed { 0. } else { self.panel_rest_w(p) };
        self.review.split.anims.retain(|(q, _)| *q != p);
        self.review.split.drag = Some(SplitDrag {
            pane: p,
            collapsed,
            want,
        });
        cx.notify()
    }

    pub(crate) fn move_panel_drag(
        &mut self,
        at: Point<Pixels>,
        viewport_w: Pixels,
        cx: &mut Context<Self>,
    ) {
        let Some(d) = self.review.split.drag.as_mut() else {
            return;
        };
        d.want = f32::from(match d.pane {
            Pane::Tree | Pane::Sidebar => at.x,
            Pane::Right | Pane::Preview => viewport_w - at.x,
        });
        cx.notify()
    }

    pub(crate) fn end_panel_drag(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.review.split.drag.take() else {
            return;
        };
        let (p, rm) = (d.pane, reduced_motion());
        let l = self.panel_limits(p);
        let drawn = self.panel_shown(p).unwrap_or_else(|| self.panel_rest_w(p));
        if d.collapsed {
            if release_collapsed(d.want) {
                self.set_panel_open(p, true, cx);
                let to = self.panel_rest_w(p);
                self.review
                    .split
                    .anims
                    .push((p, PanelAnim::new(drawn, to, false, rm)));
            } else {
                self.review
                    .split
                    .anims
                    .push((p, PanelAnim::new(drawn, 0., true, rm)));
            }
        } else {
            match release(d.want, l) {
                Release::Collapse => self
                    .review
                    .split
                    .anims
                    .push((p, PanelAnim::new(drawn, 0., true, rm))),
                Release::Width(w) => {
                    match p {
                        Pane::Tree => self.review.tree_w = gpui::px(w) / crate::ui::theme::zoom(),
                        Pane::Right => {
                            self.review.right_ratio =
                                layout::right_ratio(self.review_content_w(), gpui::px(w))
                        }
                        Pane::Sidebar | Pane::Preview => {}
                    }
                    if (drawn - w).abs() > 0.5 {
                        self.review
                            .split
                            .anims
                            .push((p, PanelAnim::new(drawn, w, false, rm)));
                    }
                }
            }
            self.settings.review_tree_w = f32::from(self.review.tree_w);
            self.settings.review_right_ratio = self.review.right_ratio;
            self.save_settings(cx);
        }
        cx.notify()
    }

    /// Start of the window render: drawn widths from the drag or the springs; a finished collapse closes
    /// the panel (the same state the toggles use).
    pub(crate) fn split_frame(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        let mut shown = vec![];
        if let Some(d) = self.review.split.drag {
            let w = if d.collapsed {
                d.want.clamp(0., self.panel_rest_w(d.pane))
            } else {
                drag_width(d.want, self.panel_limits(d.pane))
            };
            shown.push((d.pane, w));
        }
        if let Some((name, w)) = crate::dev::drag() {
            let p = Pane::named(&name);
            let l = self.panel_limits(p);
            shown.push((p, drag_width(w, l)));
        }
        self.detect_toggles(now);
        let mut closed = vec![];
        let mut opacity = vec![];
        self.review.split.anims.retain_mut(|(p, a)| {
            let f = a.frame(now);
            if f.moving {
                shown.push((*p, f.width));
                opacity.push((*p, f.opacity));
            } else if a.closing {
                closed.push(*p);
            }
            f.moving
        });
        self.dev_panel_progress(&mut shown, &mut opacity);
        for p in closed {
            self.set_panel_open(p, false, cx);
        }
        self.persist_panels(cx);
        let moving = !self.review.split.anims.is_empty();
        self.review.split.shown = shown;
        self.review.split.opacity = opacity;
        if moving {
            window.request_animation_frame();
        }
    }
}
