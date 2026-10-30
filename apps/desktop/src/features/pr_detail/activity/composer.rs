//! Comment composer pinned under the detail panel.

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::GitViewer;
use crate::ui::molecules::Textarea;
use crate::state::DetailState;
use crate::features::keymap;
use crate::state::host_avatar_url;
use crate::ui::atoms::{Avatar, Button, ButtonVariant, Kbd};
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
        let cancel = Button::new("cancel-mode", "Cancel").variant(ButtonVariant::Ghost).on_click(cx.listener(
            |this, _: &ClickEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.editing = None;
                    d.reply_to = None;
                }
                this.composer.update(cx, |i, cx| i.set_text("", cx));
                cx.notify()
            },
        ));
        let empty = self.composer.read(cx).text().trim().is_empty();
        let me = d.me.clone().unwrap_or_default();
        let avatar_url = host_avatar_url(d.summary.id.repo.kind, &d.summary.id.repo.host, &me);
        let post = Button::new("post", label).variant(ButtonVariant::Primary);
        let post = if empty || d.posting { post } else { post.on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.post_comment(cx))) };
        let footer = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .child(div().flex_1().min_w(theme::SPACE_0).text_size(theme::text_xs()).text_color(theme::text_muted()).children(mode.map(|m| div().truncate().child(m))))
            .when(d.editing.is_some() || d.reply_to.is_some(), |f| f.child(cancel))
            .child(Kbd::new(keymap::display("cmd-enter")))
            .child(div().when(empty || d.posting, |b| b.opacity(0.5)).child(post));
        // Full width under the tab pill, never overlapped by it.
        div()
            .flex()
            .gap(theme::SPACE_3)
            .pt(theme::SPACE_3)
            .pb(theme::SPACE_4)
            .border_t_1()
            .border_color(theme::border_subtle())
            .child(div().pt(theme::SPACE_2).child(Avatar::new(if me.is_empty() { "you".to_string() } else { me }).size(theme::SPACE_6).image(self.avatar(avatar_url.as_deref()))))
            .child(div().flex().flex_col().flex_1().min_w(theme::SPACE_0).gap(theme::SPACE_2).child(Textarea::new(self.composer.clone()).rows(1)).child(footer))
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
