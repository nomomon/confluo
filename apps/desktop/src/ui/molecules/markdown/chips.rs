//! Inline code chips: a rounded fill painted behind code runs (GitLab style). Text run backgrounds
//! fill the whole line box with square corners; this paints a 4px-radius rect per wrapped row,
//! padded 3px sideways and inset vertically so the line height does not grow.

use gpui::{
    canvas, div, fill, point, prelude::*, px, size, AnyElement, Bounds, Pixels, TextLayout,
};
use std::ops::Range;

use crate::ui::theme;

const PAD_X: f32 = 3.;
const RADIUS: f32 = 4.;
/// Vertical inset as a share of the line height (keeps chips off the neighbouring lines).
const INSET: f32 = 0.08;

/// One chip per visual row: `(y, x_start, x_end)` from the caret positions of a range's boundaries.
/// The last glyph of a row has no right boundary on that row, so it gets the row's mean advance.
pub(super) fn chip_rows(points: &[(f32, f32)]) -> Vec<(f32, f32, f32)> {
    let mut rows: Vec<(f32, Vec<f32>)> = vec![];
    for &(x, y) in points {
        match rows.last_mut() {
            Some((ry, xs)) if (*ry - y).abs() < 0.5 => xs.push(x),
            _ => rows.push((y, vec![x])),
        }
    }
    let n = rows.len();
    rows.into_iter()
        .enumerate()
        .filter_map(|(i, (y, xs))| {
            let (lo, hi) = (xs.first()?.min(*xs.last()?), xs.first()?.max(*xs.last()?));
            // A wrapped row ends on a glyph start: add one advance.
            let wrapped = i + 1 < n;
            let adv = if xs.len() > 1 {
                (hi - lo) / (xs.len() - 1) as f32
            } else {
                0.
            };
            let hi = if wrapped { hi + adv } else { hi };
            (hi > lo).then_some((y, lo, hi))
        })
        .collect()
}

/// `el` with chips painted behind the code `ranges` of `layout`.
pub(super) fn with_chips(
    el: impl IntoElement,
    layout: TextLayout,
    ranges: Vec<Range<usize>>,
    text: String,
) -> AnyElement {
    if ranges.is_empty() {
        return el.into_any_element();
    }
    let chips = canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            let lh: Pixels = layout.line_height();
            for r in &ranges {
                let points: Vec<(f32, f32)> = (r.start..=r.end)
                    .filter(|&i| text.is_char_boundary(i))
                    .filter_map(|i| layout.position_for_index(i))
                    .map(|p| (f32::from(p.x), f32::from(p.y)))
                    .collect();
                for (y, x0, x1) in chip_rows(&points) {
                    let inset = f32::from(lh) * INSET;
                    let b = Bounds::new(
                        point(px(x0 - PAD_X), px(y + inset)),
                        size(px(x1 - x0 + 2. * PAD_X), lh - px(2. * inset)),
                    );
                    window.paint_quad(fill(b, theme::code_inline_bg()).corner_radii(px(RADIUS)));
                }
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    div()
        .relative()
        .min_w(theme::SPACE_0)
        .child(chips)
        .child(el)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::chip_rows;

    #[test]
    fn one_row_spans_its_boundaries() {
        assert_eq!(
            chip_rows(&[(10., 0.), (18., 0.), (26., 0.)]),
            vec![(0., 10., 26.)]
        );
    }

    #[test]
    fn wrapped_range_splits_per_row() {
        let rows = chip_rows(&[(90., 0.), (98., 0.), (0., 20.), (8., 20.)]);
        assert_eq!(rows, vec![(0., 90., 106.), (20., 0., 8.)]);
    }

    #[test]
    fn empty_range_paints_nothing() {
        assert!(chip_rows(&[(4., 0.)]).is_empty());
    }
}
