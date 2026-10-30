//! A timeline event: opened, a review verdict, merged or closed.

use gpui::{div, prelude::*, SharedString};
use gv_engine::{ReviewState, TimelineEvent, TimelineKind};

use crate::app::GitViewer;
use crate::features::shared::relative_time;
use crate::ui::atoms::{tooltip, Icon, IconName};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_event(&self, e: &TimelineEvent) -> impl IntoElement {
        let (icon, color, text) = match &e.kind {
            TimelineKind::Opened => (
                IconName::GitPullRequest,
                theme::success(),
                "opened this pull request",
            ),
            TimelineKind::Reviewed(ReviewState::Approved) => {
                (IconName::Check, theme::success(), "approved these changes")
            }
            TimelineKind::Reviewed(ReviewState::ChangesRequested) => {
                (IconName::X, theme::danger(), "requested changes")
            }
            TimelineKind::Reviewed(_) => (IconName::MessageCircle, theme::text_muted(), "reviewed"),
            TimelineKind::Merged => (
                IconName::GitMerge,
                theme::merged(),
                "merged this pull request",
            ),
            TimelineKind::Closed => (
                IconName::GitPullRequestClosed,
                theme::danger(),
                "closed this pull request",
            ),
        };
        let who = if e.actor.username.is_empty() || e.actor.username == "unknown" {
            String::new()
        } else {
            e.actor
                .name
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| e.actor.username.clone())
        };
        let exact =
            e.at.with_timezone(&chrono::Local)
                .format("%b %-d, %Y %H:%M")
                .to_string();
        let id = SharedString::from(format!("ev-{}-{}", e.at.timestamp(), e.actor.username));
        div()
            .flex()
            .items_center()
            .gap(theme::space_3())
            .text_size(theme::text_sm())
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .size(theme::space_6())
                    .rounded(theme::RADIUS_FULL)
                    .bg(self.surfaces().panel())
                    .border_1()
                    .border_color(theme::border_subtle())
                    .child(Icon::new(icon).size(theme::icon_sm()).color(color)),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(theme::space_1p5())
                    .min_w(theme::space_0())
                    .text_color(theme::text_muted())
                    .when(!who.is_empty(), |d| {
                        d.child(
                            div()
                                .font_weight(theme::WEIGHT_MEDIUM)
                                .text_color(theme::text())
                                .child(who),
                        )
                    })
                    .child(text)
                    .child(
                        div()
                            .id(id)
                            .text_color(theme::text_subtle())
                            .tooltip(tooltip(exact))
                            .child(relative_time(e.at)),
                    ),
            )
    }
}
