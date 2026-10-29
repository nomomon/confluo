//! Multi-line element for `TextInput::multiline`: soft-wrapped paragraphs, cursor and selection.
//!
//! Wrapping uses the width from the previous frame, so the first frame may not wrap.

use gpui::{
    fill, point, prelude::*, px, relative, size, App, Bounds, ElementId, ElementInputHandler, Entity, GlobalElementId, Hsla, LayoutId,
    PaintQuad, Pixels, Point, SharedString, Style, TextAlign, TextRun, Window, WrappedLine,
};

use super::TextInput;
use crate::ui::theme;

/// Visible rows before the area stops growing.
const MAX_ROWS: usize = 8;

/// A shaped paragraph: byte offset of its start, top offset and layout.
#[derive(Clone)]
pub(super) struct Para {
    pub start: usize,
    pub top: Pixels,
    pub line: WrappedLine,
}

pub(super) struct AreaElement {
    pub(super) input: Entity<TextInput>,
}

pub(super) struct AreaPrepaint {
    paras: Vec<Para>,
    quads: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
}

impl IntoElement for AreaElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

fn shape(input: &TextInput, width: Option<Pixels>, window: &mut Window) -> (Vec<Para>, bool) {
    let style = window.text_style();
    let placeholder = input.content.is_empty();
    let (text, color): (SharedString, Hsla) = if placeholder { (input.placeholder.clone(), theme::TEXT_MUTED) } else { (input.content.clone(), style.color) };
    let run = TextRun { len: text.len(), font: style.font(), color, background_color: None, underline: None, strikethrough: None };
    let font_size = style.font_size.to_pixels(window.rem_size());
    let lh = window.line_height();
    let lines = window.text_system().shape_text(text, font_size, &[run], width, None).unwrap_or_default();
    let (mut start, mut top, mut out) = (0, px(0.), vec![]);
    for line in lines {
        let rows = line.wrap_boundaries().len() + 1;
        let len = line.len();
        out.push(Para { start, top, line });
        start += len + 1;
        top += lh * rows as f32;
    }
    (out, placeholder)
}

/// Point of a byte offset relative to the element origin.
pub(super) fn position(paras: &[Para], offset: usize, lh: Pixels) -> Point<Pixels> {
    let p = paras.iter().rev().find(|p| p.start <= offset).or(paras.first());
    p.and_then(|p| p.line.position_for_index(offset - p.start, lh).map(|pt| point(pt.x, pt.y + p.top))).unwrap_or_default()
}

/// Byte offset closest to a point relative to the element origin.
pub(super) fn index_at(paras: &[Para], at: Point<Pixels>, lh: Pixels) -> usize {
    let Some(p) = paras.iter().rev().find(|p| p.top <= at.y).or(paras.first()) else { return 0 };
    let i = p.line.closest_index_for_position(point(at.x, at.y - p.top), lh).unwrap_or_else(|i| i);
    p.start + i.min(p.line.len())
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
    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        let input = self.input.read(cx);
        let width = input.last_bounds.map(|b| b.size.width);
        let (paras, _) = shape(input, width, window);
        let rows: usize = paras.iter().map(|p| p.line.wrap_boundaries().len() + 1).sum();
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = (window.line_height() * rows.clamp(1, MAX_ROWS) as f32).into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, b: Bounds<Pixels>, _: &mut (), window: &mut Window, cx: &mut App) -> AreaPrepaint {
        let input = self.input.read(cx);
        let (paras, placeholder) = shape(input, Some(b.size.width), window);
        let lh = window.line_height();
        let sel = input.selected_range.clone();
        let at = |o: usize| b.origin + position(&paras, o, lh);
        if placeholder || sel.is_empty() {
            let c = if placeholder { b.origin } else { at(input.cursor_offset()) };
            let cursor = fill(Bounds::new(c, size(px(1.5), lh)), theme::ACCENT);
            return AreaPrepaint { paras, quads: vec![], cursor: Some(cursor) };
        }
        let (s, e) = (at(sel.start), at(sel.end));
        let color = theme::ACCENT.opacity(0.3);
        let quads = if s.y == e.y {
            vec![fill(Bounds::from_corners(s, point(e.x, e.y + lh)), color)]
        } else {
            vec![
                fill(Bounds::from_corners(s, point(b.right(), s.y + lh)), color),
                fill(Bounds::from_corners(point(b.left(), s.y + lh), point(b.right(), e.y)), color),
                fill(Bounds::from_corners(point(b.left(), e.y), point(e.x, e.y + lh)), color),
            ]
        };
        AreaPrepaint { paras, quads, cursor: None }
    }
    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, b: Bounds<Pixels>, _: &mut (), pre: &mut AreaPrepaint, window: &mut Window, cx: &mut App) {
        let focus = self.input.read(cx).focus_handle.clone();
        window.handle_input(&focus, ElementInputHandler::new(b, self.input.clone()), cx);
        for q in pre.quads.drain(..) {
            window.paint_quad(q);
        }
        let lh = window.line_height();
        for p in &pre.paras {
            p.line.paint(point(b.left(), b.top() + p.top), lh, TextAlign::Left, None, window, cx).ok();
        }
        if focus.is_focused(window) {
            if let Some(c) = pre.cursor.take() {
                window.paint_quad(c);
            }
        }
        let paras = std::mem::take(&mut pre.paras);
        let empty = self.input.read(cx).content.is_empty();
        self.input.update(cx, |input, _| {
            input.last_paras = if empty { vec![] } else { paras };
            input.last_bounds = Some(b);
            input.last_line_height = lh;
        });
    }
}
