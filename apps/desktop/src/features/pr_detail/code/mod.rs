//! Code tab: toolbar, optional file tree and the virtualized diff of every changed file.

mod line;
mod line_composer;
mod rows;
mod thread_card;
mod toolbar;
mod tree;
mod viewer;

use gpui::{div, list, prelude::*, AnyElement, Context, Div, Hsla};
use gv_engine::FileStatus;

use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::{ErrorText, Icon, IconName};
use crate::ui::molecules::EmptyState;
use crate::ui::theme;

/// Color for a file's status: added green, deleted red, renamed amber.
pub(super) fn status_color(s: FileStatus) -> Hsla {
    match s {
        FileStatus::Added => theme::success(),
        FileStatus::Deleted => theme::danger(),
        FileStatus::Renamed => theme::warning(),
        FileStatus::Modified => theme::text_muted(),
    }
}

/// `+a -d` in green and red, omitting zero sides.
pub(super) fn diff_counts(a: u32, d: u32) -> Div {
    div()
        .flex()
        .flex_shrink_0()
        .gap(theme::SPACE_1)
        .font_family(theme::mono_font())
        .text_size(theme::text_xs())
        .when(a > 0, |el| el.child(div().text_color(theme::success()).child(format!("+{a}"))))
        .when(d > 0, |el| el.child(div().text_color(theme::danger()).child(format!("-{d}"))))
}

/// Message icon with the number of inline threads on a file; nothing when zero.
pub(super) fn comment_count(n: usize) -> Option<Div> {
    (n > 0).then(|| {
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(theme::SPACE_0P5)
            .px(theme::SPACE_1P5)
            .rounded(theme::RADIUS_FULL)
            .bg(theme::tint_bg(theme::accent()))
            .text_color(theme::accent())
            .text_size(theme::text_2xs())
            .font_weight(theme::WEIGHT_MEDIUM)
            .child(Icon::new(IconName::Message).size(theme::ICON_SM).color(theme::accent()))
            .child(n.to_string())
    })
}

impl GitViewer {
    pub(super) fn render_code(&self, d: &DetailState, cx: &mut Context<Self>) -> AnyElement {
        let c = &d.code;
        if let Some(v) = &c.viewer {
            return self.render_file_viewer(v, cx);
        }
        let body: AnyElement = match &c.files {
            None if c.error.is_some() => ErrorText::new(c.error.clone().unwrap_or_default()).into_any_element(),
            None if c.show_skeleton => skeleton().into_any_element(),
            None => div().into_any_element(),
            Some(f) if f.is_empty() => EmptyState::new("No changed files", "This change has no file diffs").into_any_element(),
            Some(_) => {
                let rows = list(c.list.clone(), cx.processor(|this, ix: usize, _, cx| this.render_code_row(ix, cx))).flex_1().h_full();
                div()
                    .flex()
                    .flex_1()
                    .min_h(theme::SPACE_0)
                    .gap(theme::SPACE_3)
                    .when(self.code_tree, |el| el.child(self.render_file_tree(d, cx)))
                    .child(div().flex().flex_col().flex_1().min_w(theme::SPACE_0).h_full().pr(theme::SPACE_0P5).child(rows))
                    .into_any_element()
            }
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h(theme::SPACE_0)
            .gap(theme::SPACE_3)
            .child(self.render_code_toolbar(d, cx))
            .child(body)
            .into_any_element()
    }
}

/// Placeholder file cards while the first load is slow.
fn skeleton() -> impl IntoElement {
    let bar = |w: f32| div().h(theme::SPACE_3).w(gpui::relative(w)).rounded(theme::RADIUS_SM).bg(theme::surface_hover());
    let card = |id: usize| {
        div()
            .id(("code-skeleton", id))
            .flex()
            .flex_col()
            .gap(theme::SPACE_2)
            .p(theme::SPACE_4)
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(theme::border_subtle())
            .child(bar(0.4))
            .child(bar(0.9))
            .child(bar(0.75))
            .child(bar(0.6))
    };
    div().flex().flex_col().gap(theme::SPACE_3).children((0..3).map(card))
}
