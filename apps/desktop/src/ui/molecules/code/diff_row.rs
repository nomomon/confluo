//! One diff or file line: old and new line number gutters, a +/- marker and code.
//!
//! Added lines get a green tint and a 2px green accent, removed lines a red tint.

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
    /// Highlighted as part of a selected range.
    selected: bool,
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
        }
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
        let num = |n: Option<u32>| {
            div()
                .flex_shrink_0()
                .w(self.gutter_w)
                .pr(theme::SPACE_2)
                .flex()
                .justify_end()
                .text_color(theme::text_subtle())
                .child(n.map(|n| n.to_string()).unwrap_or_default())
        };
        let marker_x = theme::SPACE_0P5 + self.gutter_w * if self.show_marker { 2. } else { 1. };
        let dot = div().absolute().left(theme::SPACE_1).top((self.height - theme::SPACE_1P5) / 2.).size(theme::SPACE_1P5).rounded(theme::RADIUS_FULL).bg(theme::accent());
        div()
            .relative()
            .flex()
            .items_center()
            .w_full()
            .h(self.height)
            .overflow_hidden()
            .font_family(theme::mono_font())
            .text_size(theme::text_xs())
            .when_some(bg, |d, c| d.bg(c))
            .when(self.selected, |d| d.bg(theme::tint_bg(theme::accent())))
            .when(self.dot, |d| d.child(dot))
            .child(div().flex_shrink_0().w(theme::SPACE_0P5).h_full().when_some(accent, |d, c| d.bg(c)))
            .when(self.show_marker, |d| d.child(num(self.old_no)))
            .child(num(self.new_no))
            .when(self.show_marker, |d| {
                d.child(div().flex_shrink_0().w(theme::SPACE_4).text_color(accent.unwrap_or(theme::text_subtle())).child(marker))
            })
            .child(
                div()
                    .flex_1()
                    .min_w(theme::SPACE_0)
                    .overflow_hidden()
                    .pl(theme::SPACE_1)
                    .child(div().ml(-self.scroll_x).child(CodeText::new(self.text, self.lang))),
            )
            .when_some(self.action, |d, a| d.child(div().absolute().top_0().h_full().left(marker_x).flex().items_center().child(a)))
    }
}
