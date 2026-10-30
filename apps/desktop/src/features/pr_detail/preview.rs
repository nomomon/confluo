//! Triage preview: a slim pane beside the full-width list with just enough to decide what to
//! review next. Enter, a double click on the row or "Open review" enters Review mode.

use gpui::{div, prelude::*, ClickEvent, Context, MouseButton, MouseDownEvent};

use super::header::stats;
use crate::app::GitViewer;
use crate::features::keymap::{self, OpenInBrowser, OpenSelected, ToggleDetail};
use crate::features::shared::{layout, number_label, provider_mark, state_icon};
use crate::state::DetailState;
use crate::ui::atoms::{tooltip, Avatar, Button, ButtonVariant, IconName};
use crate::ui::{motion, theme};

/// The start of the description as markdown: up to `max_lines` non-blank source lines, HTML comments
/// dropped, an open code fence closed. Also says whether anything was cut.
pub(super) fn excerpt(md: &str, max_lines: usize) -> (String, bool) {
    let mut out = vec![];
    let (mut kept, mut in_comment, mut fences) = (0, false, 0);
    let mut lines = md.lines();
    for line in lines.by_ref() {
        let t = line.trim();
        if in_comment || t.starts_with("<!--") {
            in_comment = !t.contains("-->");
            continue;
        }
        if kept == max_lines {
            if t.is_empty() {
                continue;
            }
            if fences % 2 == 1 {
                out.push("```");
            }
            return (out.join("\n").trim().to_string(), true);
        }
        if t.starts_with("```") {
            fences += 1;
        }
        if !t.is_empty() {
            kept += 1;
        }
        out.push(line);
    }
    if fences % 2 == 1 {
        out.push("```");
    }
    (out.join("\n").trim().to_string(), false)
}

impl GitViewer {
    pub(crate) fn render_preview(
        &self,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let s = &d.summary;
        let url = s.web_url.clone();
        let repo = s.id.repo.clone();
        let desc = d
            .detail
            .as_ref()
            .map(|x| excerpt(&x.description, 6))
            .filter(|e| !e.0.is_empty());
        let avatars = s.reviewers.iter().map(|r| {
            div()
                .id(gpui::SharedString::from(format!(
                    "pv-rev-{}",
                    r.user.username
                )))
                .tooltip(tooltip(format!("{} ({:?})", r.user.username, r.state)))
                .child(
                    Avatar::new(r.user.username.clone())
                        .image(self.avatar(r.user.avatar_url.as_deref())),
                )
        });
        let meta = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(theme::SPACE_2)
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
            .child(provider_mark(s.id.repo.kind))
            .child(
                div()
                    .id("preview-repo")
                    .tooltip(tooltip(format!(
                        "{}  right-click to rename",
                        s.id.repo.path
                    )))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                            this.open_repo_menu(repo.clone(), ev.position, cx)
                        }),
                    )
                    .child(format!(
                        "{} {}",
                        self.repo_label(&s.id.repo),
                        number_label(s)
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_1)
                    .child(
                        Avatar::new(s.author.username.clone())
                            .image(self.avatar(s.author.avatar_url.as_deref())),
                    )
                    .child(s.author.username.clone()),
            );
        let status = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_y(theme::SPACE_2)
            .gap(theme::SPACE_3)
            .child(stats(s))
            .children(self.render_ci(d, "preview", cx))
            .children(self.approvals(d))
            .children(self.render_review_state(d))
            .child(div().flex_1())
            .child(div().flex().children(avatars));
        let actions = div()
            .flex()
            .gap(theme::SPACE_2)
            .child(
                Button::new("preview-review", "Open review")
                    .variant(ButtonVariant::Primary)
                    .icon(IconName::Code)
                    .tooltip(keymap::tip::<OpenSelected>("Open review"))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.enter_review(cx))),
            )
            .child(
                self.hinted::<OpenInBrowser>(
                    Button::new("preview-browser", "Open in browser")
                        .icon(IconName::ExternalLink)
                        .tooltip(keymap::tip::<OpenInBrowser>("Open in browser"))
                        .on_click(move |_, _, cx| cx.open_url(&url)),
                    crate::ui::atoms::HintAnchor::Corner,
                ),
            );
        let body = crate::ui::elastic::content("preview")
            .flex()
            .flex_col()
            .gap(theme::SPACE_4)
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(theme::SPACE_2)
                    .child(div().pt(theme::SPACE_0P5).child(state_icon(s.state)))
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme::SPACE_0)
                            .text_size(theme::text_lg())
                            .font_weight(theme::WEIGHT_SEMIBOLD)
                            .child(s.title.clone()),
                    )
                    .child(
                        self.hinted::<ToggleDetail>(
                            Button::new("preview-hide", "")
                                .icon(IconName::LayoutSidebarRight)
                                .variant(ButtonVariant::Ghost)
                                .tooltip(keymap::tip::<ToggleDetail>("Hide preview"))
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.toggle_detail(cx)
                                })),
                            crate::ui::atoms::HintAnchor::Corner,
                        ),
                    ),
            )
            .child(meta)
            .child(status)
            .when_some(desc, |p, (md, cut)| {
                // Rendered markdown, clipped to about six lines with a fade into the panel.
                let bg = self.surfaces().panel();
                let fade = div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(theme::SPACE_10)
                    .bg(gpui::linear_gradient(
                        180.,
                        gpui::linear_color_stop(bg.opacity(0.), 0.),
                        gpui::linear_color_stop(bg, 1.),
                    ));
                p.child(
                    div()
                        .relative()
                        .max_h(layout::PREVIEW_EXCERPT_H)
                        .overflow_hidden()
                        .text_color(theme::text_muted())
                        .child(self.markdown("preview-desc", &md))
                        .when(cut, |d| d.child(fade)),
                )
            })
            .child(actions);
        let h = crate::ui::elastic::handle("preview");
        let panel = div()
            .id("preview")
            .track_scroll(&h)
            .w(layout::PREVIEW_W)
            .flex_shrink_0()
            .h_full()
            .map(crate::ui::scroll::y_only)
            .bg(self.surfaces().panel())
            .border_l_1()
            .border_color(theme::border_subtle())
            .pt(layout::TITLEBAR_INSET)
            .px(theme::pane_pad())
            .pb(theme::SPACE_6)
            .child(body);
        let panel = crate::ui::elastic::y_scroller(panel, "preview", h);
        motion::slide_in_right(panel, "preview-panel", theme::SPACE_6)
    }
}

#[cfg(test)]
mod tests {
    use super::excerpt;

    #[test]
    fn excerpt_keeps_markdown_drops_comments_and_closes_fences() {
        assert_eq!(
            excerpt("<!-- tpl\nx -->\n## Summary\n\nAdds **fast** path.", 6),
            ("## Summary\n\nAdds **fast** path.".to_string(), false)
        );
        let (md, cut) = excerpt("a\nb\n```rs\nc\nd\ne\nf\ng", 6);
        assert!(cut);
        assert_eq!(md, "a\nb\n```rs\nc\nd\ne\n```");
    }
}
