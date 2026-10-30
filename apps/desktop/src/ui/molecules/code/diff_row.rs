//! One diff or file line: old and new line number gutters, a +/- marker and code.
//!
//! Added lines get a green tint and a 2px green accent, removed lines a red tint;
//! selected lines add an accent tint on top and turn the bar accent.

use gpui::{div, prelude::*, px, AnyElement, App, Pixels, SharedString, Window};

use super::{CodeText, Lang};
use crate::ui::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LineMark {
    Context,
    Add,
    Del,
}

#[derive(IntoElement)]
pub struct DiffRow {
    old_no: Option<u32>,
    new_no: Option<u32>,
    mark: LineMark,
    text: SharedString,
    lang: Lang,
    gutter_w: Pixels,
    height: Pixels,
    scroll_x: Pixels,
    show_marker: bool,
    /// Element over the marker column, e.g. a hover "+" comment button.
    action: Option<AnyElement>,
    /// Small dot in the gutter: the line has comments.
    dot: bool,
    /// Selected: `line_selected_bg()` over the add/del tint and a 2px accent bar.
    selected: bool,
    /// Side-by-side cell: one number gutter (whichever side is set) and the marker.
    one_number: bool,
    /// Find matches in the code (`CodeText::marks`).
    marks: Vec<(std::ops::Range<usize>, bool)>,
    /// Wrap the code; `height` becomes the minimum and the row grows.
    wrap: bool,
}

impl DiffRow {
    pub fn new(mark: LineMark, text: impl Into<SharedString>, lang: Lang) -> Self {
        Self {
            old_no: None,
            new_no: None,
            mark,
            text: text.into(),
            lang,
            gutter_w: theme::SPACE_10,
            height: theme::SPACE_5,
            scroll_x: px(0.),
            show_marker: true,
            action: None,
            dot: false,
            selected: false,
            one_number: false,
            marks: vec![],
            wrap: false,
        }
    }

    pub fn marks(mut self, marks: Vec<(std::ops::Range<usize>, bool)>) -> Self {
        self.marks = marks;
        self
    }

    /// Wrap long code lines instead of scrolling them (the row grows).
    pub fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
        self
    }

    /// Line numbers; pass `None` for the side a line does not exist on.
    pub fn numbers(mut self, old_no: Option<u32>, new_no: Option<u32>) -> Self {
        self.old_no = old_no;
        self.new_no = new_no;
        self
    }

    /// File view: one gutter (the new number) and no marker column.
    pub fn single_gutter(mut self) -> Self {
        self.show_marker = false;
        self
    }

    /// Side-by-side cell: a single number gutter plus the +/- marker.
    pub fn split_cell(mut self) -> Self {
        self.one_number = true;
        self
    }

    pub fn gutter_w(mut self, w: Pixels) -> Self {
        self.gutter_w = w;
        self
    }

    pub fn height(mut self, h: Pixels) -> Self {
        self.height = h;
        self
    }

    pub fn action(mut self, el: impl IntoElement) -> Self {
        self.action = Some(el.into_any_element());
        self
    }

    pub fn dot(mut self, on: bool) -> Self {
        self.dot = on;
        self
    }

    pub fn selected(mut self, on: bool) -> Self {
        self.selected = on;
        self
    }

    /// Horizontal scroll offset applied to the code column.
    pub fn scroll_x(mut self, x: Pixels) -> Self {
        self.scroll_x = x;
        self
    }
}

impl RenderOnce for DiffRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (bg, accent, marker) = match self.mark {
            LineMark::Add => (Some(theme::diff_add_bg()), Some(theme::success()), "+"),
            LineMark::Del => (Some(theme::diff_del_bg()), Some(theme::danger()), "-"),
            LineMark::Context => (None, None, " "),
        };
        let line_h = self.height;
        let num = |n: Option<u32>| {
            div()
                .flex_shrink_0()
                .w(self.gutter_w)
                .h(line_h)
                .flex()
                .items_center()
                .pr(theme::SPACE_2)
                .flex()
                .justify_end()
                .text_color(theme::text_subtle())
                .child(n.map(|n| n.to_string()).unwrap_or_default())
        };
        let marker_x = theme::SPACE_0P5
            + self.gutter_w
                * if self.show_marker && !self.one_number {
                    2.
                } else {
                    1.
                };
        let dot = div()
            .absolute()
            .left(theme::SPACE_1)
            .top((self.height - theme::SPACE_1P5) / 2.)
            .size(theme::SPACE_1P5)
            .rounded(theme::RADIUS_FULL)
            .bg(theme::accent());
        let wrap = self.wrap;
        div()
            .relative()
            .flex()
            .w_full()
            .map(|d| {
                if wrap {
                    d.items_start().min_h(self.height)
                } else {
                    d.items_center().h(self.height)
                }
            })
            .overflow_hidden()
            .font_family(theme::mono_font())
            .text_size(theme::text_xs())
            .when_some(bg, |d, c| d.bg(c))
            .when(self.selected, |d| {
                d.child(div().absolute().inset_0().bg(theme::line_selected_bg()))
            })
            .when(self.dot, |d| d.child(dot))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(theme::SPACE_0P5)
                    .when_some(
                        if self.selected {
                            Some(theme::accent())
                        } else {
                            accent
                        },
                        |d, c| d.bg(c),
                    ),
            )
            .child(div().flex_shrink_0().w(theme::SPACE_0P5))
            .when(self.show_marker && !self.one_number, |d| {
                d.child(num(self.old_no))
            })
            .child(num(if self.one_number {
                self.new_no.or(self.old_no)
            } else {
                self.new_no
            }))
            .when(self.show_marker, |d| {
                d.child(
                    div()
                        .flex_shrink_0()
                        .w(theme::SPACE_4)
                        .text_color(accent.unwrap_or(theme::text_subtle()))
                        .child(marker),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_w(theme::SPACE_0)
                    .overflow_hidden()
                    .pl(theme::SPACE_1)
                    .when(wrap, |d| d.pr(theme::SPACE_2))
                    .min_h(line_h)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .when(!wrap, |d| d.ml(-self.scroll_x))
                            .when(wrap, |d| d.w_full())
                            .child(
                                CodeText::new(self.text, self.lang)
                                    .marks(self.marks)
                                    .wrap(wrap),
                            ),
                    ),
            )
            .when_some(self.action, |d, a| {
                d.child(
                    div()
                        .absolute()
                        .top_0()
                        .h_full()
                        .left(marker_x)
                        .flex()
                        .items_center()
                        .child(a),
                )
            })
    }
}
