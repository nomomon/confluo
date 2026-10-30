//! Activity tab: a calm timeline of comments, threads and events, and the pinned composer.

mod composer;
mod event;
mod thread;

use chrono::{DateTime, Utc};
use gpui::{div, prelude::*, ClickEvent, Context};
use gv_engine::{Attention, ChangeRequestDetail, Thread, TimelineEvent};

use crate::app::GitViewer;
use crate::features::keymap;
use crate::state::{attention, DetailState, ThreadFilter};
use crate::ui::atoms::{Button, ButtonVariant, Icon, IconName};
use crate::ui::molecules::{EmptyState, ToggleGroup};
use crate::ui::theme;

enum Item<'a> {
    Thread(&'a Thread, Attention),
    Event(&'a TimelineEvent),
}

impl Item<'_> {
    fn at(&self) -> DateTime<Utc> {
        match self {
            Item::Thread(t, _) => t
                .comments
                .first()
                .map(|c| c.created_at)
                .unwrap_or_else(Utc::now),
            Item::Event(e) => e.at,
        }
    }
}

/// Threads and events in time order (a pending comment sorts last).
fn timeline<'a>(
    det: &'a ChangeRequestDetail,
    attn: &[Attention],
    filter: ThreadFilter,
) -> Vec<Item<'a>> {
    let threads = det
        .threads
        .iter()
        .zip(attn.iter().copied())
        .filter(|(_, a)| filter.keeps(*a))
        .map(|(t, a)| Item::Thread(t, a));
    // Events only in the full timeline; the filters list threads alone.
    let events = det
        .events
        .iter()
        .filter(|_| filter == ThreadFilter::All)
        .map(Item::Event);
    let mut v: Vec<Item> = threads.chain(events).collect();
    v.sort_by_key(|i| i.at());
    v
}

impl GitViewer {
    pub(super) fn render_threads(
        &self,
        det: &ChangeRequestDetail,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if det.threads.is_empty() && det.events.len() <= 1 {
            return div()
                .child(
                    EmptyState::new("No comments yet", "Start the conversation below")
                        .child(Icon::new(IconName::MessageCircle).color(theme::text_subtle())),
                )
                .into_any_element();
        }
        let attn = attention(d);
        let filter = self.review.thread_filter;
        let mut col = div()
            .relative()
            .flex()
            .flex_col()
            .gap(theme::section_gap())
            .pt(theme::space_2());
        let items = timeline(det, &attn, filter);
        let none = items.is_empty();
        for item in items {
            col = col.child(match item {
                Item::Thread(t, a) => self.render_thread(t, a, d, cx).into_any_element(),
                Item::Event(e) => self.render_event(e).into_any_element(),
            });
        }
        if none && filter != ThreadFilter::All {
            col = col.child(
                div()
                    .pl(theme::space_3())
                    .text_size(theme::text_sm())
                    .text_color(theme::text_subtle())
                    .child(if filter == ThreadFilter::NeedsMe {
                        "Nothing needs you"
                    } else {
                        "No unresolved threads"
                    }),
            );
        }
        if det.threads.is_empty() {
            col = col.child(
                div()
                    .pl(theme::space_3())
                    .text_size(theme::text_sm())
                    .text_color(theme::text_subtle())
                    .child("No comments yet"),
            );
        }
        let resolvable = attn.iter().any(|a| *a != Attention::Plain);
        div()
            .flex()
            .flex_col()
            .gap(theme::space_3())
            .when(resolvable, |c| c.child(self.thread_filter_bar(&attn, cx)))
            .child(col)
            .into_any_element()
    }

    /// All / Unresolved / Needs me with counts, and "Next unresolved" (`u`, Shift+U goes back).
    fn thread_filter_bar(&self, attn: &[Attention], cx: &mut Context<Self>) -> impl IntoElement {
        let f = self.review.thread_filter;
        let unresolved = attn.iter().filter(|a| a.unresolved()).count();
        let needs = attn.iter().filter(|a| **a == Attention::NeedsMe).count();
        let all = attn.len();
        let group = ToggleGroup::new("thread-filter")
            .item(format!("All {all}"), None, f == ThreadFilter::All)
            .item(
                format!("Unresolved {unresolved}"),
                None,
                f == ThreadFilter::Unresolved,
            )
            .item(
                format!("Needs me {needs}"),
                None,
                f == ThreadFilter::NeedsMe,
            )
            .on_change(cx.listener(|this, i: &usize, _, cx| {
                this.review.thread_filter = [
                    ThreadFilter::All,
                    ThreadFilter::Unresolved,
                    ThreadFilter::NeedsMe,
                ][*i];
                cx.notify()
            }));
        let next = Button::new("next-unresolved", "Next unresolved")
            .icon(IconName::ChevronDown)
            .variant(ButtonVariant::Ghost)
            .tooltip(keymap::tip::<keymap::NextUnresolved>("Next unresolved"))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.jump_unresolved(1, cx)));
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(theme::space_2())
            .child(group)
            .when(unresolved > 0, |r| r.child(next))
    }
}
