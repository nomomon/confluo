//! One thread in the Conversation: the shared thread container (`thread_view`) plus the scroll marker
//! "Next unresolved" uses, and a focus ring on the thread it last jumped to.

use gpui::{canvas, div, point, prelude::*, Context};
use gv_engine::{Attention, Thread};

use super::super::comment::Place;
use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_thread(
        &self,
        t: &Thread,
        a: Attention,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // "Next unresolved" scrolls the Conversation to this thread on its next paint.
        let (reveal, handle, id) = (
            self.review.reveal_conv.clone(),
            self.review.right_scroll.clone(),
            t.id.clone(),
        );
        let marker = canvas(
            move |b, window, _| {
                if reveal.borrow().as_deref() == Some(id.as_str()) {
                    reveal.borrow_mut().take();
                    let view = handle.bounds();
                    let off = handle.offset();
                    handle.set_offset(point(
                        off.x,
                        off.y - (b.origin.y - view.origin.y) + theme::SPACE_3,
                    ));
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let current = self.review.attn_at.as_deref() == Some(t.id.as_str());
        let view = self
            .thread_view(t, a, d, Place::Activity, cx)
            .when(current, |e| e.border_color(theme::focus_ring()));
        div().relative().child(marker).child(view)
    }
}
