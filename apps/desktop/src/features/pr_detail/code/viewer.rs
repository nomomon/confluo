//! Whole-file viewer: mono lines with a line number gutter, virtualized, with line selection.

use gpui::{
    div, prelude::*, uniform_list, AnyElement, ClickEvent, Context, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent,
};

use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::FileView;
use crate::ui::atoms::{Button, ButtonVariant, ErrorText, IconName};
use crate::ui::molecules::code::{DiffRow, Lang, LineMark};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_file_viewer(&self, v: &FileView, cx: &mut Context<Self>) -> AnyElement {
        let bar = div()
            .flex()
            .items_center()
            .gap(theme::space_2())
            .child(
                Button::new("viewer-back", "Back to diff")
                    .icon(IconName::ArrowLeft)
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        if let Some(d) = this.detail.as_mut() {
                            d.code.viewer = None;
                        }
                        cx.notify()
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(theme::space_0())
                    .truncate()
                    .font_family(theme::mono_font())
                    .text_size(theme::text_xs())
                    .child(v.path.clone()),
            )
            .when(v.goto, |d| {
                // Ctrl+G: line number, Enter jumps, Esc closes.
                d.child(
                    div()
                        .w(theme::space_16() * 2.)
                        .child(self.goto_input.clone()),
                )
            });
        let body: AnyElement = match (&v.lines, &v.error) {
            (_, Some(e)) => ErrorText::new(e.clone()).into_any_element(),
            (None, None) => div()
                .text_size(theme::text_sm())
                .text_color(theme::text_muted())
                .child("Loading file...")
                .into_any_element(),
            (Some(lines), None) => {
                let lines = lines.clone();
                let lang = Lang::from_path(&v.path);
                let digits = lines.len().to_string().len() as f32;
                let gutter = (theme::text_xs() * layout::MONO_CHAR_RATIO * digits
                    + theme::space_4())
                .max(layout::DIFF_GUTTER_W);
                uniform_list(
                    "file-viewer",
                    lines.len(),
                    cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                        let Some(c) = this.detail.as_ref().map(|d| &d.code) else {
                            return vec![];
                        };
                        let sel = c.viewer.as_ref().and_then(|v| v.sel);
                        let (dragging, kbd) = (c.dragging, c.sel_kbd);
                        range
                            .map(|i| {
                                let row = DiffRow::new(LineMark::Context, lines[i].clone(), lang)
                                    .numbers(None, Some(i as u32 + 1))
                                    .single_gutter()
                                    .gutter_w(gutter)
                                    .height(layout::diff_line_h())
                                    .selected(sel.is_some_and(|r| r.contains(i)));
                                let bar = sel
                                    .filter(|r| r.hi() == i && !dragging)
                                    .map(|_| this.selection_bar(false, !kbd, cx));
                                div()
                                    .id(("view-line", i))
                                    .relative()
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            move |this, ev: &MouseDownEvent, window, cx| {
                                                cx.stop_propagation();
                                                window.focus(&this.focus);
                                                this.press_view_line(
                                                    i,
                                                    ev.modifiers.shift,
                                                    false,
                                                    cx,
                                                )
                                            },
                                        ),
                                    )
                                    .on_mouse_move(cx.listener(
                                        move |this, ev: &MouseMoveEvent, _, cx| {
                                            match ev.pressed_button {
                                                Some(MouseButton::Left) => {
                                                    this.press_view_line(i, false, true, cx)
                                                }
                                                _ => this.end_line_drag(cx),
                                            }
                                        },
                                    ))
                                    .child(row)
                                    .children(bar)
                            })
                            .collect()
                    }),
                )
                .track_scroll(v.scroll.clone())
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.clear_line_selection(cx);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseUpEvent, _, cx| this.end_line_drag(cx)),
                )
                .flex_1()
                .h_full()
                .py(theme::space_2())
                .rounded(theme::RADIUS_LG)
                .border_1()
                .border_color(theme::border_subtle())
                .bg(self.surfaces().card())
                .into_any_element()
            }
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h(theme::space_0())
            .gap(theme::space_3())
            .child(bar)
            .child(body)
            .into_any_element()
    }
}
