//! Which comment shows its hover toolbar (Slack style). The toolbar sits above the row's top edge,
//! so the hover region is the row plus the toolbar: leaving either starts a grace timer
//! (`timing::HOVER_GRACE`), entering either cancels it, entering another row switches at once.
//! An open layer of the comment (More menu, picker) keeps the toolbar up; see `visible`.

use gpui::Context;

use super::comment_key::CommentKey;
use super::timing;
use crate::app::GitViewer;

#[derive(Default, Debug)]
pub struct CommentHover {
    shown: Option<CommentKey>,
    over_row: bool,
    over_bar: bool,
    /// Bumped on every change; a grace timer only hides for its own generation.
    gen: u64,
}

impl CommentHover {
    /// Pointer entered or left the row (`hovered`) of `k`. Returns a generation to arm a timer for.
    pub fn row(&mut self, k: &CommentKey, hovered: bool) -> Option<u64> {
        self.gen += 1;
        if hovered {
            if self.shown.as_ref() != Some(k) {
                self.shown = Some(k.clone());
                self.over_bar = false;
            }
            self.over_row = true;
            return None;
        }
        if self.shown.as_ref() == Some(k) {
            self.over_row = false;
            return self.arm();
        }
        None
    }

    /// Pointer entered or left the toolbar of `k`.
    pub fn bar(&mut self, k: &CommentKey, hovered: bool) -> Option<u64> {
        self.gen += 1;
        if hovered {
            self.shown = Some(k.clone());
            self.over_bar = true;
            return None;
        }
        if self.shown.as_ref() == Some(k) {
            self.over_bar = false;
            return self.arm();
        }
        None
    }

    fn arm(&self) -> Option<u64> {
        (!self.over_row && !self.over_bar).then_some(self.gen)
    }

    /// The grace timer of generation `gen` fired. Returns whether the toolbar hid.
    pub fn expire(&mut self, gen: u64) -> bool {
        if gen != self.gen || self.over_row || self.over_bar || self.shown.is_none() {
            return false;
        }
        self.shown = None;
        true
    }

    /// Whether `k` shows its toolbar; `layer_open` is true while its own menu or picker is open.
    pub fn visible(&self, k: &CommentKey, layer_open: bool) -> bool {
        layer_open || self.shown.as_ref() == Some(k)
    }
}

impl GitViewer {
    pub(crate) fn hover_comment_row(&mut self, k: &CommentKey, on: bool, cx: &mut Context<Self>) {
        let g = self.comment_hover.row(k, on);
        self.after_comment_hover(g, cx);
    }

    pub(crate) fn hover_comment_bar(&mut self, k: &CommentKey, on: bool, cx: &mut Context<Self>) {
        let g = self.comment_hover.bar(k, on);
        self.after_comment_hover(g, cx);
    }

    fn after_comment_hover(&mut self, gen: Option<u64>, cx: &mut Context<Self>) {
        if let Some(gen) = gen {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(timing::HOVER_GRACE).await;
                this.update(cx, |v, cx| {
                    if v.comment_hover.expire(gen) {
                        cx.notify();
                    }
                })
                .ok();
            })
            .detach();
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::super::comment_key::Surface;
    use super::*;

    fn k(id: &str) -> CommentKey {
        CommentKey::new(Surface::Code, id)
    }

    #[test]
    fn moving_from_row_to_toolbar_keeps_it() {
        let mut h = CommentHover::default();
        assert_eq!(h.row(&k("a"), true), None);
        // Leaving the row's top edge arms a timer, entering the toolbar cancels it.
        let g = h.row(&k("a"), false).unwrap();
        assert_eq!(h.bar(&k("a"), true), None);
        assert!(!h.expire(g));
        assert!(h.visible(&k("a"), false));
        // Reverse event order (bar enter before row leave) arms nothing.
        assert_eq!(h.row(&k("a"), false), None);
    }

    #[test]
    fn leaving_hides_after_grace_only() {
        let mut h = CommentHover::default();
        h.row(&k("a"), true);
        let g = h.row(&k("a"), false).unwrap();
        assert!(h.visible(&k("a"), false));
        assert!(h.expire(g));
        assert!(!h.visible(&k("a"), false));
    }

    #[test]
    fn re_entering_within_grace_cancels() {
        let mut h = CommentHover::default();
        h.row(&k("a"), true);
        let g = h.row(&k("a"), false).unwrap();
        h.row(&k("a"), true);
        assert!(!h.expire(g));
        assert!(h.visible(&k("a"), false));
    }

    #[test]
    fn another_row_switches_at_once() {
        let mut h = CommentHover::default();
        h.row(&k("a"), true);
        h.row(&k("b"), true);
        // a's late leave does not touch b.
        assert_eq!(h.row(&k("a"), false), None);
        assert!(h.visible(&k("b"), false));
        assert!(!h.visible(&k("a"), false));
    }

    #[test]
    fn open_menu_keeps_toolbar() {
        let mut h = CommentHover::default();
        h.row(&k("a"), true);
        h.bar(&k("a"), true);
        let g = h.bar(&k("a"), false);
        let g = g.or(h.row(&k("a"), false)).unwrap();
        assert!(h.expire(g));
        assert!(h.visible(&k("a"), true));
    }
}
