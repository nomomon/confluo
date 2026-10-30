//! Code tab: toolbar, optional file tree and the virtualized diff of every changed file.

mod context_menus;
mod find_bar;
mod line;
mod line_composer;
mod rows;
mod selection_bar;
mod thread_card;
mod toolbar;
mod tree;
mod viewer;

use gpui::{
    div, list, prelude::*, AnyElement, Context, Div, Hsla, MouseButton, MouseMoveEvent,
    MouseUpEvent,
};
use gv_engine::FileStatus;

use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::{ErrorText, Icon, IconName};
use crate::ui::molecules::EmptyState;
use crate::ui::theme;

/// Color for a file's status: added green, deleted red, renamed amber.
pub(in crate::features::pr_detail) fn status_color(s: FileStatus) -> Hsla {
    match s {
        FileStatus::Added => theme::success(),
        FileStatus::Deleted => theme::danger(),
        FileStatus::Renamed => theme::warning(),
        FileStatus::Modified => theme::text_muted(),
    }
}

/// `+a -d` in green and red, omitting zero sides.
pub(in crate::features::pr_detail) fn diff_counts(a: u32, d: u32) -> Div {
    div()
        .flex()
        .flex_shrink_0()
        .gap(theme::SPACE_1)
        .font_family(theme::mono_font())
        .text_size(theme::text_xs())
        .when(a > 0, |el| {
            el.child(div().text_color(theme::success()).child(format!("+{a}")))
        })
        .when(d > 0, |el| {
            el.child(div().text_color(theme::danger()).child(format!("-{d}")))
        })
}

/// Message icon with the number of inline threads on a file; nothing when zero.
pub(in crate::features::pr_detail) fn comment_count(n: usize) -> Option<Div> {
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
            .child(
                Icon::new(IconName::Message)
                    .size(theme::ICON_SM)
                    .color(theme::accent()),
            )
            .child(n.to_string())
    })
}

impl GitViewer {
    pub(in crate::features::pr_detail) fn render_code(
        &self,
        d: &DetailState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &d.code;
        if let Some(v) = &c.viewer {
            return self.render_file_viewer(v, cx);
        }
        let body: AnyElement = match &c.files {
            None if c.error.is_some() => {
                ErrorText::new(c.error.clone().unwrap_or_default()).into_any_element()
            }
            None if c.show_skeleton => skeleton().into_any_element(),
            None => div().into_any_element(),
            Some(f) if f.is_empty() => {
                EmptyState::new("No changed files", "This change has no file diffs")
                    .into_any_element()
            }
            Some(_) => {
                // Read the viewport here: inside the item callback the list state is mutably borrowed
                // (calling `viewport_bounds()` there panics, which aborted the app).
                let view_h = c.list.viewport_bounds().size.height;
                let rows = list(
                    c.list.clone(),
                    cx.processor(move |this, ix: usize, _, cx| {
                        this.render_code_row(ix, view_h, cx)
                    }),
                )
                .flex_1()
                .h_full();
                // Rubber band: a translate of the whole list, the list's own scroll state stays in range.
                let rows = rows.relative().top(crate::ui::elastic::shift("diff-list"));
                let band = c.list.clone();
                let drawn = f32::from(c.list.scroll_px_offset_for_scrollbar().y);
                let find = self.render_find_bar(cx);
                div()
                    .relative()
                    .flex()
                    .flex_1()
                    .min_h(theme::SPACE_0)
                    .gap(theme::SPACE_3)
                    .child(
                        div()
                            .id("diff-list")
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(theme::SPACE_0)
                            .h_full()
                            .pr(theme::SPACE_0P5)
                            .overflow_hidden()
                            .on_scroll_wheel(move |ev, window, _| {
                                if crate::ui::elastic::guard_vertical(ev, &band, drawn)
                                    && crate::ui::elastic::wheel(
                                        "diff-list",
                                        crate::ui::scroll::Axis::Y,
                                        ev,
                                        &band,
                                    )
                                {
                                    window.refresh();
                                }
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.clear_line_selection(cx);
                                }),
                            )
                            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                                if ev.pressed_button == Some(MouseButton::Left) {
                                    this.drag_autoscroll(ev.position.y, cx)
                                }
                            }))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseUpEvent, window, cx| {
                                    this.finish_line_drag(window, cx)
                                }),
                            )
                            .child(rows),
                    )
                    .children(find)
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
    let bar = |w: f32| {
        div()
            .h(theme::SPACE_3)
            .w(gpui::relative(w))
            .rounded(theme::RADIUS_SM)
            .bg(theme::surface_hover())
    };
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
    div()
        .flex()
        .flex_col()
        .gap(theme::SPACE_3)
        .children((0..3).map(card))
}
