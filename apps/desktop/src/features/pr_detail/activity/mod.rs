//! Activity tab: comment threads and the composer.

mod composer;
mod thread;

use gpui::{div, prelude::*, Context};
use gv_engine::ChangeRequestDetail;

use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_threads(&self, det: &ChangeRequestDetail, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let mut col = div().flex().flex_col().gap(theme::SPACE_3);
        if det.threads.is_empty() {
            col = col.child(div().text_size(theme::text_sm()).text_color(theme::TEXT_SUBTLE).child("No comments yet"));
        }
        for t in &det.threads {
            col = col.child(self.render_thread(t, d, cx));
        }
        col
    }
}
