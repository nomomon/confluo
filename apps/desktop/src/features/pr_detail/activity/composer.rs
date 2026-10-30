//! Comment composer pinned under the detail panel.

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::GitViewer;
use crate::ui::molecules::Textarea;
use crate::state::DetailState;
use crate::ui::atoms::{Button, ButtonVariant};
use crate::ui::theme;

impl GitViewer {
    pub(in crate::features::pr_detail) fn render_composer(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = if d.editing.is_some() {
            Some("Editing your comment".to_string())
        } else {
            d.reply_to.as_ref().map(|tid| reply_label(d, tid))
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
            .border_color(theme::border())
            .when_some(mode, |c, m| {
                c.child(div().flex().gap(theme::SPACE_2).text_size(theme::text_xs()).text_color(theme::text_muted()).child(div().flex_1().min_w(theme::SPACE_0).truncate().child(m)).child(cancel))
            })
            .child(
                div()
                    .flex()
                    .gap(theme::SPACE_2)
                    .items_end()
                    .child(div().flex_1().child(Textarea::new(self.composer.clone()).rows(2)))
                    .child(Button::new("post", label).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.post_comment(cx)))),
            )
    }
}

/// "Replying to @user: > first line of the parent comment".
fn reply_label(d: &DetailState, thread_id: &str) -> String {
    let parent = d.detail.as_ref().and_then(|det| det.threads.iter().find(|t| t.id == thread_id)).and_then(|t| t.comments.first());
    match parent {
        Some(c) => format!("Replying to @{}: > {}", c.author.username, c.body.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim()),
        None => "Replying to thread".into(),
    }
}
