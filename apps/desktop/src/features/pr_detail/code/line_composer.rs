//! New inline comment composer under a diff line, and the shared input with Cancel / submit buttons.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, Div};
use gv_engine::DiffSide;

use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::{Button, ButtonVariant, Size};
use crate::ui::molecules::Textarea;
use crate::ui::motion;
use crate::ui::theme;

impl GitViewer {
    /// The inline input followed by right-aligned Cancel and `submit` buttons.
    pub(super) fn inline_field(&self, submit: &'static str, cx: &mut Context<Self>) -> Div {
        let cancel = Button::new("inline-cancel", "Cancel").variant(ButtonVariant::Ghost).size(Size::Sm).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.cancel_inline(cx);
        }));
        let post = Button::new("inline-submit", submit).size(Size::Sm).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.submit_inline(cx)));
        div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_2)
            .child(Textarea::new(self.inline_input.clone()).rows(2).max_rows(12))
            .child(div().flex().justify_end().gap(theme::SPACE_2).child(cancel).child(post))
    }

    pub(super) fn render_line_composer(&self, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let Some(dr) = &d.code.inline.draft else { return div().into_any_element() };
        let side = if dr.side == DiffSide::Old { " (old)" } else { "" };
        let what = match dr.start {
            Some(s) => format!("Comment on lines {s}-{}{side}", dr.line),
            None => format!("Comment on line {}{side}", dr.line),
        };
        let card = div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_2)
            .p(theme::SPACE_3)
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::border())
            .bg(theme::surface())
            .child(div().text_size(theme::text_xs()).text_color(theme::text_muted()).child(what))
            .child(self.inline_field(if d.code.inline.posting { "Posting..." } else { "Comment" }, cx));
        self.card_side()
            .px(theme::SPACE_3)
            .py(theme::SPACE_2)
            .font_family(theme::ui_font())
            .child(motion::pop_in(card, "line-composer"))
            .into_any_element()
    }
}
