//! Multi-line element for `TextInput::multiline`: soft-wrapped paragraphs, caret, selection and
//! internal vertical scrolling once the content passes `max_rows`.
//!
//! Wrapping uses the width from the previous frame, so the first frame may not wrap.

use gpui::{
    fill, point, prelude::*, px, relative, size, App, Bounds, ElementId, ElementInputHandler,
    Entity, GlobalElementId, Hsla, LayoutId, LineLayout, PaintQuad, Pixels, SharedString, Style,
    TextAlign, TextRun, Window, WrappedLine,
};

use super::model::{Row, Rows};
use super::TextInput;
use crate::ui::theme;

pub(super) struct AreaElement {
    pub(super) input: Entity<TextInput>,
}

pub(super) struct AreaPrepaint {
    paras: Vec<(Pixels, WrappedLine)>,
    rows: Rows,
    quads: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
    scroll: Pixels,
}

impl IntoElement for AreaElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

/// Caret stops of one shaped line split into visual rows at `breaks` (byte offsets), offset by `base`.
pub(super) fn rows_for(layout: &LineLayout, breaks: &[usize], base: usize) -> Vec<Row> {
    let mut stops: Vec<(usize, f32)> = layout
        .runs
        .iter()
        .flat_map(|r| r.glyphs.iter().map(|g| (g.index, f32::from(g.position.x))))
        .collect();
    stops.push((layout.len, f32::from(layout.width)));
    let x_at = |i: usize| {
        stops
            .iter()
            .find(|s| s.0 >= i)
            .map(|s| s.1)
            .unwrap_or(f32::from(layout.width))
    };
    let mut edges = vec![0];
    edges.extend(breaks.iter().copied().filter(|&b| b > 0 && b < layout.len));
    edges.push(layout.len);
    edges
        .windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let x0 = x_at(a);
            let mut row: Vec<(usize, f32)> = stops
                .iter()
                .filter(|s| s.0 >= a && s.0 < b)
                .map(|s| (base + s.0, s.1 - x0))
                .collect();
            row.push((base + b, x_at(b) - x0));
            Row {
                start: base + a,
                end: base + b,
                soft: b != layout.len,
                stops: row,
            }
        })
        .collect()
}

/// Text runs for the markdown highlighting spans; gaps use the base font and color.
fn md_runs(text: &str, font: &gpui::Font, color: Hsla) -> Vec<TextRun> {
    use super::md_highlight::{spans, Mark};
    let run = |len: usize, font: gpui::Font, color: Hsla| TextRun {
        len,
        font,
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let mut out = vec![];
    let mut at = 0;
    for (r, m) in spans(text) {
        if r.start > at {
            out.push(run(r.start - at, font.clone(), color));
        }
        let mut f = font.clone();
        let c = match m {
            Mark::Heading | Mark::Bold => {
                f.weight = gpui::FontWeight::BOLD;
                color
            }
            Mark::Italic => {
                f.style = gpui::FontStyle::Italic;
                color
            }
            Mark::Syntax => theme::text_subtle(),
            Mark::Code => theme::syntax_string(),
            Mark::Link => theme::accent(),
        };
        out.push(run(r.end - r.start, f, c));
        at = r.end;
    }
    if at < text.len() || out.is_empty() {
        out.push(run(text.len() - at, font.clone(), color));
    }
    out
}

fn shape(
    input: &TextInput,
    width: Option<Pixels>,
    window: &mut Window,
) -> (Vec<(Pixels, WrappedLine)>, Rows) {
    let style = window.text_style();
    let placeholder = input.content.is_empty();
    let (text, color): (SharedString, Hsla) = if placeholder {
        (input.placeholder.clone(), theme::text_muted())
    } else {
        (input.content.clone(), style.color)
    };
    let runs = if input.markdown && !placeholder {
        md_runs(&text, &style.font(), color)
    } else {
        vec![TextRun {
            len: text.len(),
            font: style.font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }]
    };
    let font_size = style.font_size.to_pixels(window.rem_size());
    let lh = window.line_height();
    let lines = window
        .text_system()
        .shape_text(text, font_size, &runs, width, None)
        .unwrap_or_default();
    let (mut start, mut top, mut out, mut rows) = (0, px(0.), vec![], vec![]);
    for line in lines {
        let n = line.wrap_boundaries().len() + 1;
        let layout = &line.unwrapped_layout;
        let breaks: Vec<usize> = line
            .wrap_boundaries()
            .iter()
            .filter_map(|b| {
                layout
                    .runs
                    .get(b.run_ix)?
                    .glyphs
                    .get(b.glyph_ix)
                    .map(|g| g.index)
            })
            .collect();
        rows.extend(rows_for(layout, &breaks, start));
        let len = line.len();
        out.push((top, line));
        start += len + 1;
        top += lh * n as f32;
    }
    (
        out,
        if placeholder {
            Rows::default()
        } else {
            Rows(rows)
        },
    )
}

impl Element for AreaElement {
    type RequestLayoutState = ();
    type PrepaintState = AreaPrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let input = self.input.read(cx);
        let width = input.last_bounds.map(|b| b.size.width);
        let (paras, _) = shape(input, width, window);
        let rows: usize = paras.iter().map(|p| p.1.wrap_boundaries().len() + 1).sum();
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height =
            (window.line_height() * rows.clamp(input.min_rows, input.max_rows) as f32).into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        b: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> AreaPrepaint {
        let input = self.input.read(cx);
        let (paras, rows) = shape(input, Some(b.size.width), window);
        let lh = window.line_height();
        let sel = input.selected_range.clone();
        let caret = input.cursor_offset();
        let placeholder = input.content.is_empty();
        // Keep the scroll in range and, after a caret move or edit, the caret in view.
        let content_h = lh
            * paras
                .iter()
                .map(|p| p.1.wrap_boundaries().len() + 1)
                .sum::<usize>() as f32;
        let max = (content_h - b.size.height).max(px(0.));
        let mut scroll = input.scroll_y.clamp(px(0.), max);
        if input.reveal && !rows.is_empty() {
            let y = lh * rows.row_of(caret) as f32;
            if y < scroll {
                scroll = y;
            } else if y + lh > scroll + b.size.height {
                scroll = (y + lh - b.size.height).min(max);
            }
        }
        self.input.update(cx, |i, _| {
            i.scroll_y = scroll;
            i.reveal = false;
        });
        let at = |o: usize| {
            point(
                b.left() + px(rows.x_of(o)),
                b.top() + lh * rows.row_of(o) as f32 - scroll,
            )
        };
        if placeholder || sel.is_empty() {
            let c = if placeholder { b.origin } else { at(caret) };
            let cursor = fill(Bounds::new(c, size(px(1.5), lh)), theme::accent());
            return AreaPrepaint {
                paras,
                rows,
                quads: vec![],
                cursor: Some(cursor),
                scroll,
            };
        }
        let color = theme::accent().opacity(0.3);
        let quads = rows
            .0
            .iter()
            .enumerate()
            .filter(|(_, r)| r.start <= sel.end && r.end >= sel.start)
            .filter_map(|(i, r)| {
                let s = sel.start.max(r.start);
                let e = sel.end.min(r.end);
                let x0 = rows.x_of(s);
                // A selection running past a hard line end shows a small tail for the newline.
                let x1 = if sel.end > r.end && !r.soft {
                    rows.0[i].stops.last().map(|l| l.1).unwrap_or(0.) + 4.
                } else {
                    r.stops
                        .iter()
                        .find(|st| st.0 >= e)
                        .map(|st| st.1)
                        .unwrap_or(0.)
                };
                (x1 > x0 || s < e).then(|| {
                    let top = b.top() + lh * i as f32 - scroll;
                    fill(
                        Bounds::from_corners(
                            point(b.left() + px(x0), top),
                            point(b.left() + px(x1.max(x0 + 1.)), top + lh),
                        ),
                        color,
                    )
                })
            })
            .collect();
        AreaPrepaint {
            paras,
            rows,
            quads,
            cursor: None,
            scroll,
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        b: Bounds<Pixels>,
        _: &mut (),
        pre: &mut AreaPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus_handle.clone();
        window.handle_input(&focus, ElementInputHandler::new(b, self.input.clone()), cx);
        let lh = window.line_height();
        let scroll = pre.scroll;
        window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
            for q in pre.quads.drain(..) {
                window.paint_quad(q);
            }
            for (top, line) in &pre.paras {
                let y = b.top() + *top - scroll;
                if y + lh * (line.wrap_boundaries().len() + 1) as f32 >= b.top() && y <= b.bottom()
                {
                    line.paint(point(b.left(), y), lh, TextAlign::Left, None, window, cx)
                        .ok();
                }
            }
            if focus.is_focused(window) {
                if let Some(c) = pre.cursor.take() {
                    window.paint_quad(c);
                }
            }
        });
        let rows = std::mem::take(&mut pre.rows);
        self.input.update(cx, |input, _| {
            input.rows = rows;
            input.last_bounds = Some(b);
            input.last_line_height = lh;
        });
    }
}
