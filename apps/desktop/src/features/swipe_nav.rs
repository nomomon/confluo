//! Drawing a page swipe: during a swipe back the review moves with the fingers over the list view, which
//! sits dimmed at a -30% parallax offset (like iOS / macOS); a swipe forward slides the review left the
//! same way while the PR ahead comes in from the right as a placeholder until it opens.

use gpui::{div, prelude::*, px, AnyElement, Context, Hsla};

use crate::app::GitViewer;
use crate::ui::theme;

/// The page underneath trails the top one by this share of the width.
const PARALLAX: f32 = 0.3;
/// Dimming of the page underneath at the start of a swipe.
const DIM: f32 = 0.16;

impl GitViewer {
    /// `review` drawn at the page swipe offset `x` (positive = back) over what it reveals.
    pub(crate) fn render_swipe_stack(&self, review: AnyElement, x: f32, cx: &mut Context<Self>) -> AnyElement {
        let w = f32::from(self.viewport_w).max(1.);
        let t = (x.abs() / w).min(1.);
        let bg = Hsla { a: 1., ..self.surfaces().window_bg() };
        let dim = |a: f32| div().absolute().top_0().left_0().size_full().bg(Hsla::black().opacity(a));
        // Every layer keeps the full window size and only translates; an opaque floor sits under all of them.
        let page = |el: AnyElement| div().absolute().top_0().w(px(w)).h_full().bg(bg).child(el);
        let floor = || div().absolute().top_0().left_0().size_full().bg(bg);
        let edge = |left: bool| {
            // A soft shadow on the moving page's leading edge.
            let g = gpui::linear_gradient(if left { 270. } else { 90. }, gpui::linear_color_stop(Hsla::black().opacity(0.), 0.), gpui::linear_color_stop(Hsla::black().opacity(0.14), 1.));
            div().absolute().top_0().h_full().w(px(10.)).bg(g)
        };
        if x > 0. {
            let list = div()
                .flex()
                .size_full()
                .when(!self.nav.sidebar_hidden, |m| m.child(div().h_full().child(self.render_sidebar(cx))))
                .child(self.render_list(cx));
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .overflow_hidden()
                .child(floor())
                .child(page(list.into_any_element()).left(px(-PARALLAX * (w - x))))
                .child(dim(DIM * (1. - t)))
                .child(edge(true).left(px(x - 10.)))
                .child(page(review).left(px(x)))
                .into_any_element()
        } else {
            let ahead = self.nav.history.get(self.nav.pos + 1).cloned().flatten().and_then(|id| self.find_item(&id).or_else(|| self.engine.store().get_change_request(&id).ok().flatten()));
            let title = ahead.map(|c| c.title).unwrap_or_default();
            let incoming = div()
                .size_full()
                .flex()
                .flex_col()
                .pt(theme::SPACE_6)
                .pl(theme::SPACE_16)
                .text_size(theme::text_base())
                .font_weight(theme::WEIGHT_SEMIBOLD)
                .text_color(theme::text_muted())
                .child(title);
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .overflow_hidden()
                .child(floor())
                .child(page(review).left(px(x * PARALLAX)))
                .child(dim(DIM * t))
                .child(edge(false).left(px(w + x)))
                .child(page(incoming.into_any_element()).left(px(w + x)))
                .into_any_element()
        }
    }
}
