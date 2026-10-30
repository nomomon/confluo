//! Description card: markdown with a hover Edit button; editing swaps in a textarea with a
//! Write / Preview toggle, Cmd+Enter saves and Esc cancels.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};

use crate::app::GitViewer;
use crate::features::keymap;
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
            None => div().text_size(theme::text_sm()).text_color(theme::text_muted()).child(if d.loading { "Loading..." } else { "No detail available" }).into_any_element(),
            Some(det) if det.description.trim().is_empty() => div().text_size(theme::text_sm()).text_color(theme::text_subtle()).child("No description").into_any_element(),
            Some(det) => self.markdown("desc", &det.description).into_any_element(),
        };
        let edit = can_edit.then(|| {
            div().absolute().top(theme::SPACE_2).right(theme::SPACE_2).opacity(0.).group_hover("desc-card", |s| s.opacity(1.)).child(
                Button::new("desc-edit", "Edit").icon(IconName::Pencil).variant(ButtonVariant::Ghost).on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.start_body_edit(window, cx))),
            )
        });
        let reactions = d.detail.as_ref().and_then(|det| self.reactions_bar(None, &det.reactions, "desc-card".into(), d.summary.id.repo.kind != gv_engine::ProviderKind::Azure, cx));
        let body = div().flex().flex_col().gap(theme::SPACE_3).child(body).children(reactions);
        div().relative().group("desc-card").child(Card::new(self.surfaces()).child(body)).children(edit).into_any_element()
    }

    fn render_description_editor(&self, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let preview = d.ui.body_preview;
        let tabs = Tabs::new("desc-mode").variant(TabsVariant::Pill).tab("Write", !preview).tab("Preview", preview).on_select(cx.listener(|this, i: &usize, window, cx| {
            if let Some(d) = this.detail.as_mut() {
                d.ui.body_preview = *i == 1;
            }
            if *i == 0 {
                window.focus(&gpui::Focusable::focus_handle(this.body_input.read(cx), cx));
            }
            cx.notify()
        }));
        let text = self.body_input.read(cx).text();
        let content: AnyElement = if preview {
            let md = if text.trim().is_empty() { div().text_color(theme::text_subtle()).child("Nothing to preview").into_any_element() } else { self.markdown("desc-preview", &text).into_any_element() };
            div().min_h(theme::text_sm() * 1.5 * 8.).child(md).into_any_element()
        } else {
            div()
                .rounded(theme::RADIUS_MD)
                .border_1()
                .border_color(theme::border())
                .bg(theme::surface())
                .py(theme::SPACE_1)
                .child(Textarea::new(self.body_input.clone()).rows(8))
                .into_any_element()
        };
        let footer = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .child(div().flex_1().text_size(theme::text_xs()).text_color(theme::text_subtle()).child(format!("Markdown, {} to save", keymap::display("cmd-enter"))))
            .child(Button::new("desc-cancel", "Cancel").variant(ButtonVariant::Ghost).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.cancel_edits(cx);
                this.refocus = true;
            })))
            .child(Button::new("desc-save", "Save").variant(ButtonVariant::Primary).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save_body(cx))));
        Card::new(self.surfaces()).child(div().flex().child(tabs)).child(content).child(footer).into_any_element()
    }
}
