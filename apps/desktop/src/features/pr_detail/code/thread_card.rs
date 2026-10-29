//! Inline review thread under its diff line: comments, reply field, edit, resolve, and a one-line resolved state.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, Div};
use gv_engine::{Comment, Thread};

use crate::app::GitViewer;
use crate::features::shared::relative_time;
use crate::state::DetailState;
use crate::ui::atoms::{Avatar, Button, ButtonVariant, Icon, IconName, Size};
use crate::ui::molecules::markdown::markdown;
use crate::ui::theme;

fn author(c: &Comment) -> String {
    c.author.name.clone().unwrap_or_else(|| c.author.username.clone())
}

impl GitViewer {
    pub(super) fn render_inline_thread(&self, _fi: usize, ti: usize, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let Some(t) = d.detail.as_ref().and_then(|det| det.threads.get(ti)) else { return div().into_any_element() };
        let inline = &d.code.inline;
        let resolved = t.resolved == Some(true);
        let body = if resolved && !inline.expanded.contains(&t.id) {
            self.resolved_line(t, cx).into_any_element()
        } else {
            self.thread_card(t, d, cx).into_any_element()
        };
        self.card_side().px(theme::SPACE_3).py(theme::SPACE_2).font_family(theme::ui_font()).child(body).into_any_element()
    }

    fn resolved_line(&self, t: &Thread, cx: &mut Context<Self>) -> impl IntoElement {
        let who = t.comments.first().map(author).unwrap_or_default();
        let tid = t.id.clone();
        div()
            .id(gpui::SharedString::from(format!("resolved-{}", t.id)))
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .px(theme::SPACE_3)
            .py(theme::SPACE_1P5)
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::border_subtle())
            .text_size(theme::text_xs())
            .text_color(theme::text_subtle())
            .cursor_pointer()
            .hover(|s| s.bg(theme::surface_hover()))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.code.inline.expanded.insert(tid.clone());
                }
                cx.notify()
            }))
            .child(Icon::new(IconName::Check).size(theme::ICON_SM).color(theme::success()))
            .child(div().flex_1().child(format!("Resolved thread by {who}")))
            .child(format!("{} comment{}", t.comments.len(), if t.comments.len() == 1 { "" } else { "s" }))
            .child(Icon::new(IconName::ChevronDown).size(theme::ICON_SM).color(theme::text_subtle()))
    }

    fn thread_card(&self, t: &Thread, d: &DetailState, cx: &mut Context<Self>) -> Div {
        let inline = &d.code.inline;
        let pending = t.id == "pending";
        let mut card = div().flex().flex_col().rounded(theme::RADIUS_MD).border_1().border_color(theme::border()).bg(theme::surface());
        for (i, c) in t.comments.iter().enumerate() {
            card = card.child(self.inline_comment(c, d, cx).when(i > 0, |el| el.border_t_1().border_color(theme::border_subtle())));
        }
        let mut foot = div().flex().items_center().gap(theme::SPACE_2).px(theme::SPACE_3).py(theme::SPACE_2).border_t_1().border_color(theme::border_subtle());
        if inline.reply_to.as_deref() == Some(t.id.as_str()) {
            return card.child(div().p(theme::SPACE_3).border_t_1().border_color(theme::border_subtle()).child(self.inline_field("Reply", cx)));
        }
        if pending {
            return card;
        }
        let tid = t.id.clone();
        foot = foot.child(
            div()
                .id(gpui::SharedString::from(format!("reply-field-{}", t.id)))
                .flex_1()
                .px(theme::SPACE_2P5)
                .py(theme::SPACE_1P5)
                .rounded(theme::RADIUS_MD)
                .border_1()
                .border_color(theme::border())
                .text_size(theme::text_sm())
                .text_color(theme::text_subtle())
                .cursor_text()
                .hover(|s| s.bg(theme::surface_hover()))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.open_inline_field(Some(tid.clone()), None, window, cx)))
                .child("Reply..."),
        );
        if let Some(res) = t.resolved {
            let tid = t.id.clone();
            let label = if res { "Unresolve" } else { "Resolve" };
            foot = foot.child(
                Button::new(format!("resolve-{}", t.id), label)
                    .variant(ButtonVariant::Outline)
                    .size(Size::Sm)
                    .when(!res, |b| b.icon(IconName::Check))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_thread_resolved(tid.clone(), !res, cx))),
            );
        }
        card.child(foot)
    }

    fn inline_comment(&self, c: &Comment, d: &DetailState, cx: &mut Context<Self>) -> Div {
        let mine = c.id != "pending" && d.me.as_deref().is_some_and(|m| m.eq_ignore_ascii_case(&c.author.username));
        let editing = d.code.inline.editing.as_deref() == Some(c.id.as_str());
        let when = if c.id == "pending" { "posting...".to_string() } else { relative_time(c.created_at) };
        let (cid, cbody) = (c.id.clone(), c.body.clone());
        let head = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .text_size(theme::text_xs())
            .child(Avatar::new(c.author.username.clone()).image(self.avatar(c.author.avatar_url.as_deref())))
            .child(div().font_weight(theme::WEIGHT_SEMIBOLD).text_color(theme::text()).child(author(c)))
            .child(div().text_color(theme::text_subtle()).child(when))
            .child(div().flex_1())
            .when(mine && !editing, |el| {
                el.child(Button::new(format!("inline-edit-{}", c.id), "Edit").variant(ButtonVariant::Link).on_click(cx.listener(
                    move |this, _: &ClickEvent, window, cx| this.open_inline_field(None, Some((cid.clone(), cbody.clone())), window, cx),
                )))
            });
        let body = if editing {
            self.inline_field("Save", cx).into_any_element()
        } else {
            div().flex().flex_col().gap(theme::SPACE_1).text_size(theme::text_sm()).children(markdown(format!("ic-{}", c.id), &c.body)).into_any_element()
        };
        div().flex().flex_col().gap(theme::SPACE_1P5).px(theme::SPACE_3).py(theme::SPACE_2P5).child(head).child(body)
    }
}
