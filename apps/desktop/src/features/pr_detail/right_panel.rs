//! Review mode right panel: Overview (title, properties, description, checks, commits) and
//! Conversation (the activity timeline with the composer pinned below).

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};
use gv_engine::{commits_since, CommitInfo};

use crate::app::GitViewer;
use crate::features::shared::{layout, relative_time};
use crate::state::{DetailState, RightTab};
use crate::ui::atoms::{tooltip, Icon, IconName, ListItem};
use crate::ui::molecules::{Tabs, TabsVariant};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_right_panel(
        &self,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tab = self.review.right_tab;
        let threads = d.detail.as_ref().map(|x| x.threads.len()).unwrap_or(0);
        let tabs = Tabs::new("right-tabs")
            .variant(TabsVariant::Underline)
            .tab("Overview", tab == RightTab::Overview)
            .tab(
                if threads > 0 {
                    format!("Conversation {threads}")
                } else {
                    "Conversation".into()
                },
                tab == RightTab::Conversation,
            )
            .on_select(cx.listener(|this, i: &usize, _, cx| {
                this.review.right_tab = if *i == 0 {
                    RightTab::Overview
                } else {
                    RightTab::Conversation
                };
                cx.notify()
            }));
        let sw = &self.review.swipe;
        let tabs = tabs.progress(sw.panel_progress);
        // Pages side by side on a strip that follows the pager; at rest only the active one is built.
        let pw = self.right_page_w();
        let offsets = crate::ui::motion::interactive::page_offsets(sw.panel_x, pw, 2);
        // While both draw, each page is opaque (the panel surface) so the other never shows through.
        let page_bg = gpui::Hsla {
            a: 1.,
            ..self.surfaces().panel()
        };
        let both = sw.panel.moving() || crate::dev::panel().is_some();
        let mut strip = div()
            .relative()
            .flex_1()
            .min_h(theme::SPACE_0)
            .overflow_hidden();
        for (i, t) in [RightTab::Overview, RightTab::Conversation]
            .into_iter()
            .enumerate()
        {
            if !both && t != tab {
                continue;
            }
            let (key, handle) = match t {
                RightTab::Overview => (
                    "right-overview",
                    crate::ui::elastic::handle("right-overview"),
                ),
                RightTab::Conversation => ("right-scroll", self.review.right_scroll.clone()),
            };
            let body: AnyElement = match t {
                RightTab::Overview => self.render_overview(d, cx).into_any_element(),
                RightTab::Conversation => match &d.detail {
                    Some(det) => self.render_threads(det, d, cx).into_any_element(),
                    None => div()
                        .text_size(theme::text_sm())
                        .text_color(theme::text_muted())
                        .child(if d.loading {
                            "Loading..."
                        } else {
                            "No detail available"
                        })
                        .into_any_element(),
                },
            };
            let scroller = div()
                .id(key)
                .track_scroll(&handle)
                .size_full()
                .map(crate::ui::scroll::y_only)
                .px(theme::SPACE_3)
                .pt(theme::SPACE_3)
                .pb(layout::END_PAD)
                .child(crate::ui::elastic::content(key).child(body));
            strip = strip.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .w(gpui::px(pw))
                    .left(gpui::px(offsets[i]))
                    .when(both, |p| p.bg(page_bg))
                    .child(crate::ui::elastic::y_scroller(scroller, key, handle)),
            );
        }
        div()
            .id("right-panel")
            .flex()
            .flex_col()
            .size_full()
            .on_scroll_wheel(cx.listener(Self::panel_wheel))
            .child(div().px(theme::SPACE_3).pt(theme::SPACE_2).child(tabs))
            .child(strip)
            .when(tab == RightTab::Conversation, |p| {
                p.child(
                    div()
                        .px(theme::SPACE_3)
                        .pb(theme::SPACE_3)
                        .child(self.render_composer(d, cx)),
                )
            })
    }

    fn render_overview(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let has_checks = d.summary.checks.total > 0
            || d.detail.as_ref().is_some_and(|x| !x.checks.runs.is_empty());
        div()
            .flex()
            .flex_col()
            .gap(theme::card_gap())
            .child(self.render_title(d, cx))
            .child(self.render_properties(d, cx))
            .children(self.since_review(d))
            .child(self.render_description(d, cx))
            .when(has_checks, |c| c.child(self.render_checks_card(d, cx)))
            .children(
                d.detail
                    .as_ref()
                    .filter(|x| !x.commits.is_empty())
                    .map(|x| self.render_commits(&x.commits, x.my_review_sha.as_deref(), cx)),
            )
            .child(self.render_checkout_card(d, cx))
    }

    /// "3 new commits since your review" when I reviewed an earlier head.
    fn since_review(&self, d: &DetailState) -> Option<impl IntoElement> {
        let det = d.detail.as_ref()?;
        let n = commits_since(&det.commits, det.my_review_sha.as_deref()).filter(|n| *n > 0)?;
        Some(
            div()
                .flex()
                .items_center()
                .gap(theme::SPACE_2)
                .px(theme::SPACE_3)
                .py(theme::SPACE_2)
                .rounded(theme::RADIUS_MD)
                .bg(theme::tint_bg(theme::accent()))
                .text_size(theme::text_xs())
                .text_color(theme::accent())
                .child(
                    Icon::new(IconName::GitCommit)
                        .size(theme::ICON_SM)
                        .color(theme::accent()),
                )
                .child(if n == 1 {
                    "1 new commit since your last review".to_string()
                } else {
                    format!("{n} new commits since your last review")
                }),
        )
    }

    fn render_commits(
        &self,
        commits: &[CommitInfo],
        reviewed: Option<&str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let seen = reviewed.and_then(|r| commits.iter().position(|c| c.sha == r));
        let rows =
            commits.iter().enumerate().rev().map(|(i, c)| {
                let sha = c.sha.clone();
                let new = seen.is_some_and(|s| i > s);
                div()
                    .id(("commit", i))
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_2)
                    .px(theme::SPACE_2)
                    .py(theme::item_pad_y())
                    .list_item_radius(false, theme::RADIUS_SM)
                    .text_size(theme::text_xs())
                    .cursor_pointer()
                    .tooltip(tooltip("Show this commit's changes"))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.show_commit(sha.clone(), cx)
                    }))
                    .child(
                        div()
                            .flex_shrink_0()
                            .font_family(theme::mono_font())
                            .text_color(if new {
                                theme::accent()
                            } else {
                                theme::text_subtle()
                            })
                            .child(c.short().to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme::SPACE_0)
                            .truncate()
                            .child(c.message.clone()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_color(theme::text_subtle())
                            .child(match c.at {
                                Some(t) => format!("{} {}", c.author, relative_time(t)),
                                None => c.author.clone(),
                            }),
                    )
            });
        div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_1)
            .child(
                div()
                    .text_size(theme::text_sm())
                    .font_weight(theme::WEIGHT_MEDIUM)
                    .child(format!("Commits {}", commits.len())),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .p(theme::LIST_INSET)
                    .rounded(theme::LIST_RADIUS)
                    .border_1()
                    .border_color(theme::card_border())
                    .bg(self.surfaces().card())
                    .children(rows),
            )
    }
}
