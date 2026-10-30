//! One conversation or inline thread in the timeline: file:line chip, first comment, nested replies.

use gpui::{div, prelude::*, ClickEvent, Context};
use gv_engine::Thread;

use super::super::comment::{CommentCtx, Place};
use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::{Badge, Button, ButtonVariant, IconName};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_thread(&self, t: &Thread, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let tid = (t.id != "pending").then(|| t.id.clone());
        let mut col = div().flex().flex_col().gap(theme::SPACE_3);
        let location = t.position.as_ref().map(|p| {
            let range = p.start_line.map(|s| format!("{s}-")).unwrap_or_default();
            format!("{}:{range}{}", p.path, p.line)
        });
        let location = location.or_else(|| t.comments.first().and_then(|c| c.position.as_ref()).map(|p| format!("{}{}", p.path, p.line.map(|l| format!(":{l}")).unwrap_or_default())));
        if let Some(loc) = location {
            let show = tid.clone().filter(|_| t.position.is_some());
            col = col.child(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_2)
                    .pl(theme::SPACE_6 + theme::SPACE_3)
                    .child(div().min_w(theme::SPACE_0).truncate().px(theme::SPACE_2).py(theme::SPACE_0P5).rounded(theme::RADIUS_SM).bg(theme::surface()).text_size(theme::text_xs()).font_family(theme::mono_font()).text_color(theme::text_muted()).child(loc))
                    .when(t.outdated, |d| d.child(Badge::new("Outdated")))
                    .when(t.resolved == Some(true), |d| d.child(Badge::new("Resolved").tint(theme::success())))
                    .children(show.map(|id| {
                        Button::new(format!("show-in-code-{id}"), "Show in code").icon(IconName::Code).variant(ButtonVariant::Ghost).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_thread_in_code(id.clone(), cx)))
                    })),
            );
        }
        for (i, c) in t.comments.iter().enumerate() {
            let ctx = CommentCtx { place: Place::Activity, thread: tid.clone(), resolved: if i == 0 { t.resolved } else { None }, reply: i > 0 };
            let row = self.comment_row(c, d, ctx, cx);
            col = col.child(if i == 0 { row } else { div().pl(theme::SPACE_6 + theme::SPACE_3).child(row) });
        }
        col
    }
}
