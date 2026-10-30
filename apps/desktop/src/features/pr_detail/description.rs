//! Description card: markdown with a hover Edit button; editing swaps in a textarea with a
//! Write / Preview toggle, Cmd+Enter saves and Esc cancels. The editor keeps the rendered text's column,
//! font size, line height and height, with the controls in a footer, so opening it does not move the text.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};

use crate::app::GitViewer;
use crate::features::keymap;
use crate::features::shared::layout;
use crate::state::DetailState;
use crate::ui::atoms::{Button, ButtonVariant, IconName};
use crate::ui::molecules::{Card, Tabs, TabsVariant, Textarea};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_description(&self, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        if d.ui.editing_body {
            return self.render_description_editor(d, cx);
        }
        let can_edit = d.detail.as_ref().is_some_and(|x| x.edit.caps.body);
        let body = match &d.detail {
            None => div()
                .text_size(theme::text_sm())
                .text_color(theme::text_muted())
                .child(if d.loading {
                    "Loading..."
                } else {
                    "No detail available"
                })
                .into_any_element(),
            Some(det) if det.description.trim().is_empty() => div()
                .text_size(theme::text_sm())
                .text_color(theme::text_subtle())
                .child("No description")
                .into_any_element(),
            Some(det) => self.markdown("desc", &det.description).into_any_element(),
        };
        let edit = can_edit.then(|| {
            div()
                .absolute()
                .top(theme::SPACE_2)
                .right(theme::SPACE_2)
                .opacity(0.)
                .group_hover("desc-card", |s| s.opacity(1.))
                .child(
                    Button::new("desc-edit", "Edit")
                        .icon(IconName::Pencil)
                        .variant(ButtonVariant::Ghost)
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.start_body_edit(window, cx)
                        })),
                )
        });
        let reactions = d.detail.as_ref().and_then(|det| {
            self.reactions_bar(
                None,
                &det.reactions,
                "desc-card".into(),
                d.summary.id.repo.kind != gv_engine::ProviderKind::Azure,
                cx,
            )
        });
        let desc_h = self.desc_h.clone();
        let body = div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_3)
            .on_children_prepainted(move |b, _, _| {
                if let Some(first) = b.first() {
                    desc_h.set(first.size.height)
                }
            })
            .child(body)
            .children(reactions);
        div()
            .relative()
            .group("desc-card")
            .child(Card::new(self.surfaces()).child(body))
            .children(edit)
            .into_any_element()
    }

    fn render_description_editor(&self, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let preview = d.ui.body_preview;
        let tabs = Tabs::new("desc-mode")
            .variant(TabsVariant::Pill)
            .tab("Write", !preview)
            .tab("Preview", preview)
            .on_select(cx.listener(|this, i: &usize, window, cx| {
                if let Some(d) = this.detail.as_mut() {
                    d.ui.body_preview = *i == 1;
                }
                if *i == 0 {
                    window.focus(&gpui::Focusable::focus_handle(this.body_input.read(cx), cx));
                }
                cx.notify()
            }));
        let text = self.body_input.read(cx).text();
        // Same column, size and line height as the rendered description, and at least as tall, so nothing jumps.
        let min_h = self.desc_h.get().max(layout::DESC_EDITOR_MIN_H);
        let content: AnyElement = if preview {
            let md = if text.trim().is_empty() {
                div()
                    .text_color(theme::text_subtle())
                    .child("Nothing to preview")
                    .into_any_element()
            } else {
                self.markdown("desc-preview", &text).into_any_element()
            };
            div().min_h(min_h).child(md).into_any_element()
        } else {
            self.body_input.update(cx, |i, _| i.set_min_h(min_h));
            Textarea::new(self.body_input.clone())
                .rows(1)
                .max_rows(255)
                .into_any_element()
        };
        let footer = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .pt(theme::SPACE_2)
            .border_t_1()
            .border_color(theme::border_subtle())
            .child(tabs)
            .child(
                div()
                    .flex_1()
                    .text_size(theme::text_xs())
                    .text_color(theme::text_subtle())
                    .child(format!(
                        "Markdown, {} to save",
                        keymap::display("cmd-enter")
                    )),
            )
            .child(
                Button::new("desc-cancel", "Cancel")
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.cancel_edits(cx);
                        this.refocus = true;
                    })),
            )
            .child(
                Button::new("desc-save", "Save")
                    .variant(ButtonVariant::Primary)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save_body(cx))),
            );
        Card::new(self.surfaces())
            .child(content)
            .child(footer)
            .into_any_element()
    }
}
