//! Conversation thread header (path in mono with copy, status as text, Show in code) and the code
//! context of the commented lines, read from the cached diff.

use gpui::{div, prelude::*, ClickEvent, Context, Div};
use gv_engine::{code_context, Attention, LineKind, Thread};

use super::section;
use crate::app::GitViewer;
use crate::state::{Copied, DetailState};
use crate::ui::atoms::{Button, ButtonVariant, IconName};
use crate::ui::molecules::code::{DiffRow, Lang, LineMark};
use crate::ui::theme;

/// Lines of context above the commented range, and the most lines shown.
const CONTEXT: usize = 2;
const MAX_LINES: usize = 4;

/// `path:line` or `path:start-line` of an inline thread, else the first comment's position.
fn location(t: &Thread) -> Option<(String, String)> {
    if let Some(p) = &t.position {
        let range = p.start_line.map(|s| format!("{s}-")).unwrap_or_default();
        return Some((p.path.clone(), format!(":{range}{}", p.line)));
    }
    let p = t.comments.first()?.position.as_ref()?;
    let line = p.line.map(|l| format!(":{l}")).unwrap_or_default();
    Some((p.path.clone(), line))
}

/// Status text: amber "Needs you", muted "Unresolved" / "Outdated" / "Resolved".
fn status(a: Attention) -> Option<(&'static str, gpui::Hsla)> {
    match a {
        Attention::NeedsMe => Some(("Needs you", theme::warning())),
        Attention::Open => Some(("Unresolved", theme::text_subtle())),
        Attention::Outdated => Some(("Outdated", theme::text_subtle())),
        Attention::Resolved => Some(("Resolved", theme::text_subtle())),
        Attention::Plain => None,
    }
}

impl GitViewer {
    pub(super) fn thread_header(
        &self,
        t: &Thread,
        a: Attention,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let loc = location(t);
        let status = status(a);
        if loc.is_none() && status.is_none() {
            return None;
        }
        let show = (t.position.is_some() && t.id != "pending").then(|| t.id.clone());
        let mut row = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_1)
            .min_h(theme::control_h())
            .pl(theme::SPACE_3)
            .pr(theme::SPACE_1)
            .text_size(theme::text_xs());
        let row_has_path = loc.is_some();
        if let Some((path, line)) = loc {
            let copy = path.clone();
            row = row.child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .min_w(theme::SPACE_0)
                    .font_family(theme::mono_font())
                    .child(
                        div()
                            .min_w(theme::SPACE_0)
                            .truncate()
                            .text_color(theme::text())
                            .child(format!("{path}{line}")),
                    )
                    .child(
                        Button::new(format!("thread-copy-{}", t.id), "")
                            .icon(IconName::Copy)
                            .variant(ButtonVariant::Ghost)
                            .tooltip("Copy file path")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.copy(Copied::Path, copy.clone(), cx)
                            })),
                    ),
            );
        }
        if !row_has_path {
            row = row.child(div().flex_1());
        }
        if let Some((label, color)) = status {
            row = row.child(
                div()
                    .px(theme::SPACE_1)
                    .text_color(color)
                    .when(a == Attention::NeedsMe, |e| {
                        e.font_weight(theme::WEIGHT_MEDIUM)
                    })
                    .child(label),
            );
        }
        Some(row.children(show.map(|id| {
            // Icon only: the narrow right panel keeps its width for the path.
            Button::new(format!("show-in-code-{id}"), "")
                .icon(IconName::Code)
                .variant(ButtonVariant::Ghost)
                .tooltip("Show in code")
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.show_thread_in_code(id.clone(), cx)
                }))
        })))
    }

    /// Two to four diff lines ending at the anchor, with old/new numbers and add/del tints.
    pub(super) fn thread_snippet(&self, t: &Thread, d: &DetailState) -> Option<Div> {
        let pos = t.position.as_ref()?;
        let files = d.code.files.as_ref()?;
        let file = files
            .iter()
            .find(|f| f.new_path == pos.path || f.old_path == pos.path)?;
        let lines = code_context(file, pos, CONTEXT, MAX_LINES);
        if lines.is_empty() {
            return None;
        }
        let lang = Lang::from_path(&pos.path);
        Some(
            section()
                .py(theme::SPACE_1)
                .children(lines.into_iter().map(|l| {
                    let mark = match l.kind {
                        LineKind::Add => LineMark::Add,
                        LineKind::Del => LineMark::Del,
                        LineKind::Context => LineMark::Context,
                    };
                    DiffRow::new(mark, l.text, lang).numbers(l.old_no, l.new_no)
                })),
        )
    }
}
