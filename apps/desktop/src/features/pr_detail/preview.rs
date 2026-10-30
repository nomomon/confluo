//! Triage preview: a slim pane beside the full-width list with just enough to decide what to
//! review next. Enter, a double click on the row or "Open review" enters Review mode.

use gpui::{div, prelude::*, ClickEvent, Context, MouseButton, MouseDownEvent};

use super::header::stats;
use crate::app::GitViewer;
use crate::features::keymap::{self, OpenInBrowser, OpenSelected, ToggleDetail};
use crate::features::shared::{layout, number_label, provider_icon, state_icon};
use crate::state::DetailState;
use crate::ui::atoms::{tooltip, Avatar, Button, ButtonVariant, IconName};
use crate::ui::{motion, theme};

/// First paragraph of the description as plain text, at most `max` characters.
pub(super) fn excerpt(md: &str, max: usize) -> String {
    let text: String = md
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("<!--") && !l.starts_with('|') && !l.starts_with("```"))
        .take(6)
        .collect::<Vec<_>>()
        .join(" ");
    let clean: String = text.chars().filter(|c| !matches!(c, '*' | '`' | '_')).collect();
    if clean.chars().count() > max {
        format!("{}...", clean.chars().take(max).collect::<String>().trim_end())
    } else {
        clean
    }
}

impl GitViewer {
    pub(crate) fn render_preview(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let s = &d.summary;
        let url = s.web_url.clone();
        let repo = s.id.repo.clone();
        let desc = d.detail.as_ref().map(|x| excerpt(&x.description, 320)).filter(|e| !e.is_empty());
        let avatars = s.reviewers.iter().map(|r| {
            div()
                .id(gpui::SharedString::from(format!("pv-rev-{}", r.user.username)))
                .tooltip(tooltip(format!("{} ({:?})", r.user.username, r.state)))
                .child(Avatar::new(r.user.username.clone()).image(self.avatar(r.user.avatar_url.as_deref())))
        });
        let meta = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(theme::SPACE_2)
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
            .child(provider_icon(s.id.repo.kind))
            .child(
                div()
                    .id("preview-repo")
                    .tooltip(tooltip(format!("{}  right-click to rename", s.id.repo.path)))
                    .on_mouse_down(MouseButton::Right, cx.listener(move |this, ev: &MouseDownEvent, _, cx| this.open_repo_menu(repo.clone(), ev.position, cx)))
                    .child(format!("{} {}", self.repo_label(&s.id.repo), number_label(s))),
            )
            .child(div().flex().items_center().gap(theme::SPACE_1).child(Avatar::new(s.author.username.clone()).image(self.avatar(s.author.avatar_url.as_deref()))).child(s.author.username.clone()));
        let status = div()
            .flex()
            .items_center()
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
            .child(Button::new("preview-review", "Open review").variant(ButtonVariant::Primary).icon(IconName::Code).tooltip(keymap::tip::<OpenSelected>("Open review")).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.enter_review(cx))))
            .child(self.hinted::<OpenInBrowser>(
                Button::new("preview-browser", "Open in browser").icon(IconName::ExternalLink).tooltip(keymap::tip::<OpenInBrowser>("Open in browser")).on_click(move |_, _, cx| cx.open_url(&url)),
                crate::ui::atoms::HintAnchor::Corner,
            ));
        let panel = div()
            .id("preview")
            .flex()
            .flex_col()
            .gap(theme::SPACE_4)
            .w(layout::PREVIEW_W)
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .bg(self.surfaces().panel())
            .border_l_1()
            .border_color(theme::border_subtle())
            .pt(layout::TITLEBAR_INSET)
            .px(theme::pane_pad())
            .pb(theme::SPACE_6)
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(theme::SPACE_2)
                    .child(div().pt(theme::SPACE_0P5).child(state_icon(s.state)))
                    .child(div().flex_1().min_w(theme::SPACE_0).text_size(theme::text_lg()).font_weight(theme::WEIGHT_SEMIBOLD).child(s.title.clone()))
                    .child(self.hinted::<ToggleDetail>(
                        Button::new("preview-hide", "").icon(IconName::LayoutSidebarRight).variant(ButtonVariant::Ghost).tooltip(keymap::tip::<ToggleDetail>("Hide preview")).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_detail(cx))),
                        crate::ui::atoms::HintAnchor::Corner,
                    )),
            )
            .child(meta)
            .child(status)
            .when_some(desc, |p, t| p.child(div().text_size(theme::text_sm()).text_color(theme::text_muted()).child(t)))
            .child(actions);
        motion::slide_in_right(panel, "preview-panel", theme::SPACE_6)
    }
}

#[cfg(test)]
mod tests {
    use super::excerpt;

    #[test]
    fn excerpt_skips_headings_and_trims() {
        assert_eq!(excerpt("## Summary\n\nAdds **fast** path.\n\n- one", 100), "Adds fast path. - one");
        assert_eq!(excerpt("abcdef", 3), "abc...");
    }
}
