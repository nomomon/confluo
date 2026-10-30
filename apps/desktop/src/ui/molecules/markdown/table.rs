//! Markdown tables: header row, cell borders, column alignment and horizontal scroll.
//! Column widths come from the longest cell so rows line up and wide tables scroll.

use gpui::{div, prelude::*, px, AnyElement, ElementId, Pixels};

use super::blocks::Align;
use super::inline::Inline;
use super::render::Ctx;
use super::text::MdText;
use crate::ui::theme;

const COL_MIN: f32 = 56.;
const COL_MAX: f32 = 360.;

/// Width for a column whose longest cell has `chars` characters.
pub fn col_width(chars: usize, text_px: f32) -> Pixels {
    px((chars as f32 * text_px * 0.56 + 24.).clamp(COL_MIN, COL_MAX))
}

pub fn table(ctx: &Ctx, aligns: &[Align], head: &[Inline], rows: &[Vec<Inline>]) -> AnyElement {
    let cols = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
    let size = f32::from(theme::text_xs());
    let widths: Vec<Pixels> = (0..cols)
        .map(|i| {
            let longest = std::iter::once(head).chain(rows.iter().map(Vec::as_slice)).filter_map(|r| r.get(i)).map(|c| c.text.chars().count()).max().unwrap_or(0);
            col_width(longest, size)
        })
        .collect();
    let row = |cells: &[Inline], is_head: bool, last: bool| {
        div().flex().when(!last, |d| d.border_b_1().border_color(theme::border_subtle())).when(is_head, |d| d.bg(theme::surface())).children((0..cols).map(|i| {
            let cell = div()
                .w(widths[i])
                .flex_shrink_0()
                .flex()
                .px(theme::SPACE_3)
                .py(theme::SPACE_2)
                .when(i > 0, |d| d.border_l_1().border_color(theme::border_subtle()))
                .map(|d| match aligns.get(i) {
                    Some(Align::Center) => d.justify_center(),
                    Some(Align::Right) => d.justify_end(),
                    _ => d,
                });
            match cells.get(i) {
                Some(c) => cell.child(MdText::new(ctx.id(), c, if is_head { theme::WEIGHT_SEMIBOLD } else { theme::WEIGHT_NORMAL }, ctx.images.clone())),
                None => cell,
            }
        }))
    };
    let n = rows.len();
    let grid = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .rounded(theme::RADIUS_MD)
        .border_1()
        .border_color(theme::border_subtle())
        .overflow_hidden()
        .child(row(head, true, n == 0))
        .children(rows.iter().enumerate().map(|(i, r)| row(r, false, i + 1 == n)));
    div().id(ElementId::Name(ctx.id())).overflow_x_scroll().text_size(theme::text_xs()).child(grid).into_any_element()
}
