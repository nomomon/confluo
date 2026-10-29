//! Comment composer pinned under the detail panel.

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::{Button, ButtonVariant};
use crate::ui::theme;

impl GitViewer {
    pub(in crate::features::pr_detail) fn render_composer(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = if d.editing.is_some() {
            Some("Editing your comment")
        } else if d.reply_to.is_some() {
            Some("Replying to thread")
        } else {
            None
        };
        let label = if d.posting {
            "Posting..."
        } else if d.editing.is_some() {
            "Save"
        } else {
            "Comment"
        };
        let cancel = Button::new("cancel-mode", "Cancel").variant(ButtonVariant::Link).on_click(cx.listener(
            |this, _: &ClickEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.editing = None;
                    d.reply_to = None;
                }
                this.composer.update(cx, |i, cx| i.set_text("", cx));
                cx.notify()
            },
        ));
        div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_1)
            .pb(theme::SPACE_3)
            .pt(theme::SPACE_2)
            .border_t_1()
            .border_color(theme::BORDER)
            .when_some(mode, |c, m| c.child(div().flex().gap(theme::SPACE_2).text_size(theme::TEXT_XS).text_color(theme::TEXT_MUTED).child(m).child(cancel)))
            .child(
                div()
                    .flex()
                    .gap(theme::SPACE_2)
                    .items_center()
                    .child(div().flex_1().child(self.composer.clone()))
                    .child(Button::new("post", label).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.post_comment(cx)))),
            )
    }
}
