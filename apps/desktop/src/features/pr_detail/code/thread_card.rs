//! Inline review thread under its diff line: the shared thread container (`thread_view`) without the
//! path header or code context, since it sits under the line it is about.

use gpui::{div, prelude::*, AnyElement, Context};
use gv_engine::Attention;

use super::super::comment::Place;
use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_inline_thread(
        &self,
        _fi: usize,
        ti: usize,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(t) = d.detail.as_ref().and_then(|det| det.threads.get(ti)) else {
            return div().into_any_element();
        };
        let a = crate::state::attention(d)
            .get(ti)
            .copied()
            .unwrap_or(Attention::Plain);
        self.card_side()
            .px(theme::SPACE_3)
            .pt(theme::SPACE_7)
            .pb(theme::SPACE_3)
            .font_family(theme::ui_font())
            .child(self.thread_view(t, a, d, Place::Code, cx))
            .into_any_element()
    }
}
