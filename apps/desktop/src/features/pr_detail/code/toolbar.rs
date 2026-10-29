//! Code tab toolbar: tree toggle, prev/next file, "File i of N", collapse all and copy.

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::GitViewer;
use crate::state::code_rows::patch_text;
use crate::state::{Copied, DetailState};
use crate::ui::atoms::{Button, ButtonVariant, IconName};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_code_toolbar(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        let c = &d.code;
        let n = c.file_count();
        let icon = |id: &'static str, glyph: IconName, tip: &'static str| Button::new(id, "").icon(glyph).variant(ButtonVariant::Ghost).tooltip(tip);
        let label = match (n, c.loading) {
            (0, true) => "Loading files...".to_string(),
            (0, false) => "No files".to_string(),
            _ => format!("File {} of {n}", c.current_file() + 1),
        };
        let all_collapsed = n > 0 && c.collapsed.len() == n;
        div()
            .flex()
            .items_center()
            .gap(theme::SPACE_1)
            .child(
                icon("code-tree", IconName::LayoutSidebar, "Toggle file tree").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.code_tree = !this.code_tree;
                    cx.notify()
                })),
            )
            .child(icon("code-prev", IconName::ChevronUp, "Previous file  p").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.step_file(-1, cx))))
            .child(icon("code-next", IconName::ChevronDown, "Next file  n").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.step_file(1, cx))))
            .child(div().pl(theme::SPACE_1).text_size(theme::text_sm()).text_color(theme::text_muted()).child(label))
            .child(div().flex_1())
            .child(
                icon("code-collapse", IconName::Fold, if all_collapsed { "Expand all" } else { "Collapse all" }).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    if let Some(d) = this.detail.as_mut() {
                        d.code.toggle_all();
                    }
                    cx.notify()
                })),
            )
            .child(icon("code-copy", if self.copied == Some(Copied::Diff) { IconName::Check } else { IconName::Copy }, "Copy diff").on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    let Some(files) = this.detail.as_ref().and_then(|d| d.code.files.clone()) else { return };
                    this.copy(Copied::Diff, patch_text(&files), cx)
                },
            )))
    }

    /// Jump to the next (`1`) or previous (`-1`) file card.
    pub(crate) fn step_file(&mut self, delta: isize, cx: &mut Context<Self>) {
        if let Some(d) = self.detail.as_mut() {
            d.code.step(delta);
        }
        cx.notify()
    }

    /// Collapse or expand the file at the top of the viewport.
    pub(crate) fn toggle_current_file(&mut self, cx: &mut Context<Self>) {
        if let Some(d) = self.detail.as_mut() {
            let cur = d.code.current_file();
            d.code.toggle_file(cur);
            d.code.goto(cur);
        }
        cx.notify()
    }
}
