//! Markdown tables: header row, cell borders, column alignment and horizontal scroll.
//! Column widths come from the longest cell so rows line up and wide tables scroll.

use gpui::{canvas, div, fill, point, prelude::*, px, size, AnyElement, Bounds, Div, ElementId, Pixels, ScrollHandle, Stateful};

use super::blocks::Align;
use super::inline::Inline;
use super::render::Ctx;
use super::text::MdText;
use crate::ui::scroll::Axis;
use crate::ui::theme;

/// Horizontal scroll container around `child` that never grows past its parent: trackpad swipes and Shift+wheel scroll
/// it sideways, plain vertical wheel passes through to the page.
///
/// It lays its child out as a flex row so the child keeps its max-content width; as a block box it
/// was stretched to the container width, the content size equalled the box and there was nothing to
/// scroll (the overflow was only clipped). When the content is wider, a thin thumb along the bottom
/// edge shows that it scrolls and where it is. The scroll offset lives in a handle kept per id.
pub fn hscroll(id: ElementId, child: impl IntoElement) -> Stateful<Div> {
    let key = format!("{id:?}");
    let h = scroll_handle(&key);
    let thumb = h.clone();
    // The content moves by the rubber-band stretch at the x edges (`ui::elastic`).
    let content = div().flex_none().relative().left(crate::ui::elastic::shift(&key)).child(child);
    let mut d = div().id(id).w_full().min_w(theme::SPACE_0).flex().items_start().overflow_x_scroll().track_scroll(&h).child(content);
    d.style().restrict_scroll_to_axis = Some(true);
    // GPUI's own handler (it runs first) already moved x by the event's x delta. The gesture's owner is
    // decided once (`ui::arbiter`): the block takes a sideways gesture only if it could still scroll that way
    // when the gesture began; then it keeps the x (stretching at the edges) and stops the event. Otherwise
    // (vertical gesture, or one owned by a pager) it takes the x back and lets the event bubble.
    let (lock_key, undo) = (key.clone(), h.clone());
    d = d.on_scroll_wheel(move |ev, window, cx| {
        let delta = ev.delta.pixel_delta(window.line_height());
        let off = undo.offset();
        let before = off.x - delta.x;
        if matches!(ev.touch_phase, gpui::TouchPhase::Started) {
            crate::ui::elastic::wheel(&lock_key, Axis::X, ev, &undo);
        }
        let can = if delta.x > px(0.) { before < px(-0.5) } else { delta.x < px(0.) && before > -undo.max_offset().width + px(0.5) };
        if crate::ui::arbiter::block_handles_x(ev, &lock_key, can) {
            cx.stop_propagation();
            if crate::ui::elastic::wheel(&lock_key, Axis::X, ev, &undo) {
                window.refresh();
            }
        } else if delta.x != px(0.) {
            undo.set_offset(point(before, off.y));
        }
    });
    let debug = std::env::var_os("CONFLUO_DEBUG_LAYOUT").is_some();
    d.relative().child(
        canvas(|_, _, _| {}, move |b, _, window, _| {
            let max = thumb.max_offset().width;
            if debug {
                eprintln!("hscroll {key} box_w={} max_scroll_x={max} offset_x={}", b.size.width, thumb.offset().x);
            }
            if let Some(t) = thumb_bounds(b, max, -thumb.offset().x) {
                window.paint_quad(fill(t, theme::text_subtle().opacity(0.45)).corner_radii(t.size.height / 2.));
            }
        })
        .absolute()
        .top_0()
        .left_0()
        .size_full(),
    )
}

thread_local! {
    static HANDLES: std::cell::RefCell<std::collections::HashMap<String, ScrollHandle>> = Default::default();
}

fn scroll_handle(key: &str) -> ScrollHandle {
    HANDLES.with(|m| m.borrow_mut().entry(key.to_string()).or_default().clone())
}

const THUMB_H: f32 = 3.;

/// The scroll thumb along the bottom of a box `b` scrolled `x` of `max`; None when nothing overflows.
/// The box is drawn at its scroll offset, so the thumb is shifted back by `x` to stay in view.
pub fn thumb_bounds(b: Bounds<Pixels>, max: Pixels, x: Pixels) -> Option<Bounds<Pixels>> {
    if max <= px(1.) {
        return None;
    }
    let w = b.size.width;
    let len = (w * (w / (w + max))).max(px(24.));
    let pos = (w - len) * (x / max).clamp(0., 1.);
    Some(Bounds { origin: point(b.origin.x + pos + x, b.origin.y + b.size.height - px(THUMB_H + 1.)), size: size(len, px(THUMB_H)) })
}

const COL_MIN: f32 = 56.;
const COL_MAX: f32 = 360.;

/// Width for a column whose longest cell has `chars` characters.
pub fn col_width(chars: usize, text_px: f32) -> Pixels {
    px((chars as f32 * text_px * 0.58 + 26.).clamp(COL_MIN, COL_MAX))
}

pub fn table(ctx: &Ctx, aligns: &[Align], head: &[Inline], rows: &[Vec<Inline>]) -> AnyElement {
    let cols = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
    let size = f32::from(theme::text_xs());
    let widths: Vec<Pixels> = (0..cols)
        .map(|i| {
            let longest = std::iter::once(head).chain(rows.iter().map(Vec::as_slice)).filter_map(|r| r.get(i)).map(|c| c.text.chars().count()).max().unwrap_or(0);
            // Header cells are semibold, so they need a little more room per character.
            let head_chars = head.get(i).map(|c| c.text.chars().count() * 6 / 5 + 1).unwrap_or(0);
            let longest = longest.max(head_chars);
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
    hscroll(ElementId::Name(ctx.id()), grid).text_size(theme::text_xs()).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_only_when_content_overflows_and_tracks_offset() {
        let b = Bounds { origin: point(px(0.), px(0.)), size: size(px(500.), px(200.)) };
        assert!(thumb_bounds(b, px(0.), px(0.)).is_none());
        let start = thumb_bounds(b, px(500.), px(0.)).unwrap();
        assert_eq!(start.size.width, px(250.));
        assert_eq!(start.origin.x, px(0.));
        // Scrolled to the end: the thumb sits at the right edge of the visible box.
        let end = thumb_bounds(b, px(500.), px(500.)).unwrap();
        assert_eq!(end.origin.x - px(500.), px(250.));
    }
}
