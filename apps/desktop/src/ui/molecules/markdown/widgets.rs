//! Stateful markdown blocks: fenced code with a copy button, and collapsible `<details>`.

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, ClipboardItem, ElementId, SharedString, Window,
};

use crate::ui::atoms::{Button, ButtonVariant, Icon, IconName};
use crate::ui::molecules::code::{CodeText, Lang};
use crate::ui::theme;

/// Map a fence info string (`rust`, `ts`, `yaml`) to a tokenizer language.
pub fn fence_lang(info: &str) -> Lang {
    let ext = match info.to_ascii_lowercase().as_str() {
        "rust" => "rs".to_string(),
        "typescript" | "tsx" | "javascript" | "jsx" => "ts".into(),
        "python" => "py".into(),
        "golang" => "go".into(),
        "bash" | "shell" | "console" | "zsh" => "sh".into(),
        "dockerfile" | "docker" => return Lang::from_path("Dockerfile"),
        other => other.to_string(),
    };
    Lang::from_path(&format!("x.{ext}"))
}

#[derive(IntoElement)]
pub struct CodeBlock {
    id: SharedString,
    lang: String,
    code: String,
}

impl CodeBlock {
    pub fn new(id: SharedString, lang: &str, code: &str) -> Self {
        Self {
            id,
            lang: lang.to_string(),
            code: code.to_string(),
        }
    }
}

impl RenderOnce for CodeBlock {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let copied = window.use_keyed_state(
            ElementId::Name(format!("{}-copied", self.id).into()),
            cx,
            |_, _| false,
        );
        let done = *copied.read(cx);
        let lang = fence_lang(&self.lang);
        let code = self.code.clone();
        let group: SharedString = format!("{}-g", self.id).into();
        let copy = Button::new(format!("{}-copy", self.id), "")
            .icon(if done {
                IconName::Check
            } else {
                IconName::Copy
            })
            .variant(ButtonVariant::Ghost)
            .tooltip("Copy")
            .on_click(move |_: &ClickEvent, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(code.clone()));
                copied.update(cx, |v, cx| {
                    *v = true;
                    cx.notify()
                });
            });
        let lines = super::table::hscroll(
            ElementId::Name(format!("{}-scroll", self.id).into()),
            div()
                .flex()
                .flex_col()
                .flex_none()
                .children(self.code.lines().map(|l| {
                    div()
                        .flex_none()
                        .whitespace_nowrap()
                        .child(CodeText::new(l.to_string(), lang))
                })),
        )
        .p(theme::SPACE_3)
        .text_size(theme::text_xs());
        div()
            .group(group.clone())
            .relative()
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(theme::border_subtle())
            .bg(theme::surface())
            .child(lines)
            .child(
                div()
                    .absolute()
                    .top(theme::SPACE_1)
                    .right(theme::SPACE_1)
                    .opacity(if done { 1. } else { 0. })
                    .group_hover(group, |s| s.opacity(1.))
                    .child(copy),
            )
    }
}

#[derive(IntoElement)]
pub struct Details {
    id: SharedString,
    summary: String,
    children: Vec<AnyElement>,
}

impl Details {
    pub fn new(id: SharedString, summary: &str, children: Vec<AnyElement>) -> Self {
        Self {
            id,
            summary: summary.to_string(),
            children,
        }
    }
}

impl RenderOnce for Details {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = window.use_keyed_state(
            ElementId::Name(format!("{}-open", self.id).into()),
            cx,
            |_, _| false,
        );
        let is_open = *open.read(cx);
        let header = div()
            .id(ElementId::Name(self.id.clone()))
            .flex()
            .items_center()
            .gap(theme::SPACE_1P5)
            .cursor_pointer()
            .text_color(theme::text_muted())
            .hover(|s| s.text_color(theme::text()))
            .child(
                Icon::new(if is_open {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .size(theme::ICON_SM),
            )
            .child(div().font_weight(theme::WEIGHT_MEDIUM).child(self.summary))
            .on_click(move |_: &ClickEvent, _, cx| {
                open.update(cx, |v, cx| {
                    *v = !*v;
                    cx.notify()
                })
            });
        div()
            .flex()
            .flex_col()
            .text_size(theme::text_sm())
            .child(header)
            .when(is_open, |d| {
                d.child(
                    div()
                        .mt(theme::SPACE_2)
                        .pl(theme::SPACE_5)
                        .flex()
                        .flex_col()
                        .children(self.children),
                )
            })
    }
}
