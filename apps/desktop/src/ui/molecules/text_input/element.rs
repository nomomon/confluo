//! Custom element that shapes and paints the input line, cursor and selection.

use gpui::{
    fill, point, prelude::*, px, relative, size, App, Bounds, ElementId, ElementInputHandler, Entity, GlobalElementId, Hsla,
    LayoutId, PaintQuad, Pixels, ShapedLine, SharedString, Style, TextRun, UnderlineStyle, Window,
};

use super::TextInput;
use crate::ui::theme;

pub(super) struct TextElement {
    pub(super) input: Entity<TextInput>,
}

pub(super) struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&gpui::InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> PrepaintState {
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();
        let (text, color): (SharedString, Hsla) =
            if content.is_empty() { (input.placeholder.clone(), theme::text_muted()) } else { (content, style.color) };
        let run = TextRun { len: text.len(), font: style.font(), color, background_color: None, underline: None, strikethrough: None };
        let runs = if let Some(m) = input.marked_range.as_ref() {
            vec![
                TextRun { len: m.start, ..run.clone() },
                TextRun {
                    len: m.end - m.start,
                    underline: Some(UnderlineStyle { color: Some(run.color), thickness: px(1.0), wavy: false }),
                    ..run.clone()
                },
                TextRun { len: text.len() - m.end, ..run },
            ]
            .into_iter()
            .filter(|r| r.len > 0)
            .collect()
        } else {
            vec![run]
        };
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window.text_system().shape_line(text, font_size, &runs, None);
        let x = line.x_for_index(if input.content.is_empty() { 0 } else { cursor });
        let (selection, cursor) = if selected.is_empty() {
            (None, Some(fill(Bounds::new(point(bounds.left() + x, bounds.top()), size(px(1.5), bounds.bottom() - bounds.top())), theme::accent())))
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(bounds.left() + line.x_for_index(selected.start), bounds.top()),
                        point(bounds.left() + line.x_for_index(selected.end), bounds.bottom()),
                    ),
                    theme::accent().opacity(0.3),
                )),
                None,
            )
        };
        PrepaintState { line: Some(line), cursor, selection }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(&focus_handle, ElementInputHandler::new(bounds, self.input.clone()), cx);
        if let Some(sel) = prepaint.selection.take() {
            window.paint_quad(sel)
        }
        let line = prepaint.line.take().unwrap_or_default();
        line.paint(bounds.origin, window.line_height(), window, cx).ok();
        if focus_handle.is_focused(window) {
            if let Some(c) = prepaint.cursor.take() {
                window.paint_quad(c);
            }
        }
        let is_placeholder = self.input.read(cx).content.is_empty();
        self.input.update(cx, |input, _| {
            input.rows = if is_placeholder { super::model::Rows::default() } else { super::model::Rows(super::area::rows_for(&line, &[], 0)) };
            input.last_bounds = Some(bounds);
            input.last_line_height = bounds.size.height;
        });
    }
}
