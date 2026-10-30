//! Comment bodies with suggested changes: markdown around a "Suggested change" box that shows the
//! original line(s) against the suggestion as a mini diff. GitLab offers "Apply suggestion" (its own
//! endpoint); everywhere "Copy" puts the suggested text on the clipboard.

use gpui::{div, prelude::*, AnyElement, ClickEvent, ClipboardItem, Context};
use gv_engine::{
    split_suggestions, suggestion_original, suggestion_text, BodyPart, Comment, DiffPosition,
    Suggestion, SuggestionRef,
};

use super::section;
use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::{Button, ButtonVariant, IconName};
use crate::ui::molecules::code::{DiffRow, Lang, LineMark};
use crate::ui::theme;

impl GitViewer {
    /// The comment body: plain markdown, or text parts and suggestion boxes in order.
    pub(in crate::features::pr_detail) fn comment_body(
        &self,
        c: &Comment,
        pos: Option<&DiffPosition>,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let parts = split_suggestions(&c.body);
        if !parts.iter().any(|p| matches!(p, BodyPart::Suggestion(_))) {
            return self
                .markdown(format!("c-{}", c.id), &c.body)
                .into_any_element();
        }
        let mut col = div().flex().flex_col().gap(theme::SPACE_2);
        let mut k = 0;
        for (i, p) in parts.iter().enumerate() {
            col = col.child(match p {
                BodyPart::Text(t) => self
                    .markdown(format!("c-{}-{i}", c.id), t)
                    .into_any_element(),
                BodyPart::Suggestion(s) => {
                    let r = c.suggestions.get(k);
                    k += 1;
                    self.suggestion_box(&format!("{}-{i}", c.id), s, r, pos, d, cx)
                }
            });
        }
        col.into_any_element()
    }

    fn suggestion_box(
        &self,
        key: &str,
        s: &Suggestion,
        r: Option<&SuggestionRef>,
        pos: Option<&DiffPosition>,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let original = pos.and_then(|p| {
            let files = d.code.files.as_ref()?;
            let f = files.iter().find(|f| f.new_path == p.path)?;
            suggestion_original(f, p, s)
        });
        let lang = pos.map(|p| Lang::from_path(&p.path)).unwrap_or(Lang::Plain);
        let text = suggestion_text(s);
        let copy = Button::new(format!("sugg-copy-{key}"), "Copy")
            .icon(IconName::Copy)
            .variant(ButtonVariant::Ghost)
            .tooltip("Copy suggestion")
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                this.show_toast("Suggestion copied", cx)
            }));
        let apply = r.filter(|r| r.appliable && !r.applied).map(|r| {
            let id = r.id.clone();
            Button::new(format!("sugg-apply-{key}"), "Apply suggestion")
                .variant(ButtonVariant::Outline)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.apply_suggestion(id.clone(), cx)
                }))
        });
        let applied = r.is_some_and(|r| r.applied);
        let head = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .pl(theme::SPACE_3)
            .pr(theme::SPACE_1)
            .min_h(theme::control_h())
            .text_size(theme::text_xs())
            .child(
                div()
                    .flex_1()
                    .font_weight(theme::WEIGHT_MEDIUM)
                    .text_color(theme::text())
                    .child("Suggested change"),
            )
            .when(applied, |e| {
                e.child(div().text_color(theme::text_subtle()).child("Applied"))
            })
            .child(copy)
            .children(apply);
        let mut rows = section().py(theme::SPACE_1);
        if let Some((from, lines)) = original {
            for (j, l) in lines.into_iter().enumerate() {
                rows = rows.child(
                    DiffRow::new(LineMark::Del, l, lang).numbers(Some(from + j as u32), None),
                );
            }
            for (j, l) in s.lines.iter().enumerate() {
                rows = rows.child(
                    DiffRow::new(LineMark::Add, l.clone(), lang)
                        .numbers(None, Some(from + j as u32)),
                );
            }
        } else {
            for l in &s.lines {
                rows = rows.child(DiffRow::new(LineMark::Add, l.clone(), lang));
            }
        }
        div()
            .rounded(theme::RADIUS_SM)
            .border_1()
            .border_color(theme::border())
            .overflow_hidden()
            .child(head)
            .child(rows)
            .into_any_element()
    }
}
