//! Activity tab: a calm timeline of comments, threads and events, and the pinned composer.

mod composer;
mod event;
mod thread;

use chrono::{DateTime, Utc};
use gpui::{div, prelude::*, px, Context};
use gv_engine::{ChangeRequestDetail, Thread, TimelineEvent};

use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::{Icon, IconName};
use crate::ui::molecules::EmptyState;
use crate::ui::theme;

enum Item<'a> {
    Thread(&'a Thread),
    Event(&'a TimelineEvent),
}

impl Item<'_> {
    fn at(&self) -> DateTime<Utc> {
        match self {
            Item::Thread(t) => t.comments.first().map(|c| c.created_at).unwrap_or_else(Utc::now),
            Item::Event(e) => e.at,
        }
    }
}

/// Threads and events in time order (a pending comment sorts last).
fn timeline(det: &ChangeRequestDetail) -> Vec<Item<'_>> {
    let mut v: Vec<Item> = det.threads.iter().map(Item::Thread).chain(det.events.iter().map(Item::Event)).collect();
    v.sort_by_key(|i| i.at());
    v
}

impl GitViewer {
    pub(super) fn render_threads(&self, det: &ChangeRequestDetail, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        if det.threads.is_empty() && det.events.len() <= 1 {
            return div().child(EmptyState::new("No comments yet", "Start the conversation below").child(Icon::new(IconName::MessageCircle).color(theme::text_subtle()))).into_any_element();
        }
        // A hairline through the avatar centers ties the items together.
        let rail = div().absolute().top(theme::SPACE_3).bottom(theme::SPACE_3).left(theme::SPACE_3 - px(0.5)).w(px(1.)).bg(theme::border_subtle());
        let mut col = div().relative().flex().flex_col().gap(theme::section_gap()).pt(theme::SPACE_2).child(rail);
        for item in timeline(det) {
            col = col.child(match item {
                Item::Thread(t) => self.render_thread(t, d, cx).into_any_element(),
                Item::Event(e) => self.render_event(e).into_any_element(),
            });
        }
        if det.threads.is_empty() {
            col = col.child(div().pl(theme::SPACE_6 + theme::SPACE_3).text_size(theme::text_sm()).text_color(theme::text_subtle()).child("No comments yet"));
        }
        col.into_any_element()
    }
}
