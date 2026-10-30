//! Text input adapted from gpui's `examples/input.rs`; single-line, or multi-line
//! via `multiline()` (Enter inserts a newline, Cmd+Enter submits).
//!
//! Multi-line inputs grow from `min_rows` to `max_rows`, then scroll inside (wheel, caret kept in
//! view). Caret and selection logic lives in the pure `model` (rows of caret stops, word moves,
//! undo history) so it is unit-tested without GPUI.

mod actions;
mod area;
mod element;
mod ime;
pub(crate) mod md_edit;
pub(crate) mod md_highlight;
pub(crate) mod model;

use gpui::{
    actions, div, prelude::*, px, App, Bounds, Context, CursorStyle, EventEmitter, FocusHandle, Focusable, KeyBinding, MouseButton, Pixels,
    ScrollWheelEvent, SharedString, Window,
};
use std::ops::Range;

use crate::ui::theme;
use element::TextElement;

actions!(
    text_input,
    [
        Backspace,
        Delete,
        DeleteWordLeft,
        DeleteWordRight,
        DeleteToLineStart,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        SelectAll,
        Home,
        End,
        SelectHome,
        SelectEnd,
        DocStart,
        DocEnd,
        SelectDocStart,
        SelectDocEnd,
        Undo,
        Redo,
        Tab,
        Paste,
        Cut,
        Copy,
        Submit,
        Newline,
        Outdent,
        Bold,
        Italic,
        Link
    ]
);

pub fn bind_keys(cx: &mut App) {
    let ctx = Some("TextInput");
    let multi = Some("TextInput && multiline");
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("alt-backspace", DeleteWordLeft, ctx),
        KeyBinding::new("alt-delete", DeleteWordRight, ctx),
        KeyBinding::new("cmd-backspace", DeleteToLineStart, ctx),
        KeyBinding::new("left", Left, ctx),
        KeyBinding::new("right", Right, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new("alt-left", WordLeft, ctx),
        KeyBinding::new("alt-right", WordRight, ctx),
        KeyBinding::new("alt-shift-left", SelectWordLeft, ctx),
        KeyBinding::new("alt-shift-right", SelectWordRight, ctx),
        KeyBinding::new("cmd-a", SelectAll, ctx),
        KeyBinding::new("cmd-v", Paste, ctx),
        KeyBinding::new("cmd-c", Copy, ctx),
        KeyBinding::new("cmd-x", Cut, ctx),
        KeyBinding::new("cmd-z", Undo, ctx),
        KeyBinding::new("cmd-shift-z", Redo, ctx),
        KeyBinding::new("home", Home, ctx),
        KeyBinding::new("end", End, ctx),
        KeyBinding::new("cmd-left", Home, ctx),
        KeyBinding::new("cmd-right", End, ctx),
        KeyBinding::new("shift-home", SelectHome, ctx),
        KeyBinding::new("shift-end", SelectEnd, ctx),
        KeyBinding::new("cmd-shift-left", SelectHome, ctx),
        KeyBinding::new("cmd-shift-right", SelectEnd, ctx),
        KeyBinding::new("cmd-up", DocStart, ctx),
        KeyBinding::new("cmd-down", DocEnd, ctx),
        KeyBinding::new("cmd-shift-up", SelectDocStart, ctx),
        KeyBinding::new("cmd-shift-down", SelectDocEnd, ctx),
        KeyBinding::new("enter", Submit, ctx),
        // Later bindings win at equal depth, so multi-line inputs get a newline on Enter.
        KeyBinding::new("enter", Newline, multi),
        KeyBinding::new("up", Up, multi),
        KeyBinding::new("down", Down, multi),
        KeyBinding::new("shift-up", SelectUp, multi),
        KeyBinding::new("shift-down", SelectDown, multi),
        KeyBinding::new("tab", Tab, Some("TextInput && tabs")),
        KeyBinding::new("cmd-enter", Submit, ctx),
        // Markdown editors: these win over the app's Cmd+B / Cmd+I / Cmd+K while the editor has focus.
        KeyBinding::new("shift-tab", Outdent, Some("TextInput && markdown")),
        KeyBinding::new("cmd-b", Bold, Some("TextInput && markdown")),
        KeyBinding::new("cmd-i", Italic, Some("TextInput && markdown")),
        KeyBinding::new("cmd-k", Link, Some("TextInput && markdown")),
    ]);
}

pub enum InputEvent {
    Changed,
    Submit,
}

pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    /// Visual rows of the last frame (absolute byte offsets); empty for the placeholder.
    rows: model::Rows,
    last_bounds: Option<Bounds<Pixels>>,
    last_line_height: Pixels,
    is_selecting: bool,
    multiline: bool,
    /// Goal x for Up/Down, kept across consecutive vertical moves.
    goal_x: Option<f32>,
    history: model::History,
    min_rows: usize,
    max_rows: usize,
    /// Tab inserts two spaces (markdown editors).
    tabs: bool,
    /// Markdown assistance (list continuation, indent, Cmd+B/I/K) and highlighting.
    markdown: bool,
    /// Borderless, transparent, no inset, markdown body line height: edits in place of rendered text.
    document: bool,
    /// Minimum height of the whole field (document editors match the rendered text they replace).
    min_h: Pixels,
    scroll_y: Pixels,
    /// Scroll the caret into view on the next frame.
    reveal: bool,
    /// Left and right padding; `Input` widens them for a leading icon or clear button.
    inset: (Pixels, Pixels),
}

impl EventEmitter<InputEvent> for TextInput {}

impl TextInput {
    pub fn new(placeholder: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            placeholder: placeholder.into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            rows: model::Rows::default(),
            last_bounds: None,
            last_line_height: px(0.),
            is_selecting: false,
            multiline: false,
            goal_x: None,
            history: model::History::default(),
            min_rows: 1,
            max_rows: 12,
            tabs: false,
            markdown: false,
            document: false,
            min_h: px(0.),
            scroll_y: px(0.),
            reveal: false,
            inset: (theme::SPACE_3, theme::SPACE_3),
        }
    }

    pub fn set_placeholder(&mut self, s: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.placeholder = s.into();
        cx.notify();
    }

    pub fn text(&self) -> String {
        self.content.to_string()
    }

    pub fn set_text(&mut self, s: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = s.into();
        let n = self.content.len();
        self.selected_range = n..n;
        self.marked_range = None;
        self.history.clear();
        self.scroll_y = px(0.);
        self.reveal = true;
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    /// Room for adornments drawn over the field (see `Input`).
    pub fn inset(mut self, left: Pixels, right: Pixels) -> Self {
        self.inset = (left, right);
        self
    }

    /// Grow to several soft-wrapped lines; Enter inserts a newline.
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// Markdown editor: Tab indents, list continuation, Cmd+B / Cmd+I / Cmd+K, light highlighting.
    pub fn markdown(mut self) -> Self {
        self.tabs = true;
        self.markdown = true;
        self
    }

    /// Edit in place of rendered markdown: no border, fill or inset, and the markdown body's line height.
    pub fn document(mut self) -> Self {
        self.document = true;
        self.inset = (px(0.), px(0.));
        self
    }

    pub fn set_min_h(&mut self, h: Pixels) {
        self.min_h = h;
    }

    /// Visible rows of a multi-line input: grows from `min` to `max`, then scrolls.
    pub fn set_rows(&mut self, min: usize, max: usize) {
        self.min_rows = min.max(1);
        self.max_rows = max.max(self.min_rows);
    }

    fn on_scroll(&mut self, e: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let lh = self.last_line_height;
        let dy = e.delta.pixel_delta(lh).y;
        let content = lh * self.rows.len().max(1) as f32;
        let view = self.last_bounds.map(|b| b.size.height).unwrap_or(content);
        let max = (content - view).max(px(0.));
        if max > px(0.) {
            self.scroll_y = (self.scroll_y - dy).clamp(px(0.), max);
            cx.stop_propagation();
            cx.notify();
        }
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        let mut ctx = gpui::KeyContext::default();
        ctx.add("TextInput");
        if self.multiline {
            ctx.add("multiline");
        }
        if self.tabs {
            ctx.add("tabs");
        }
        if self.markdown {
            ctx.add("markdown");
        }
        div()
            .flex()
            .items_center()
            .key_context(ctx)
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_word_left))
            .on_action(cx.listener(Self::delete_word_right))
            .on_action(cx.listener(Self::delete_to_line_start))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::word_left))
            .on_action(cx.listener(Self::word_right))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::doc_start))
            .on_action(cx.listener(Self::doc_end))
            .on_action(cx.listener(Self::select_doc_start))
            .on_action(cx.listener(Self::select_doc_end))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::outdent))
            .on_action(cx.listener(Self::bold))
            .on_action(cx.listener(Self::italic))
            .on_action(cx.listener(Self::link))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .when(self.multiline, |d| d.on_scroll_wheel(cx.listener(Self::on_scroll)))
            .w_full()
            .when(!self.multiline, |d| d.h(theme::control_h()))
            .when(self.multiline && !self.document, |d| d.items_start().min_h(theme::control_h()).py(theme::SPACE_2))
            .when(self.document, |d| d.items_start().line_height(gpui::relative(theme::MARKDOWN_LINE_HEIGHT)))
            .min_h(self.min_h)
            .pl(self.inset.0)
            .pr(self.inset.1)
            .when(!self.document, |d| {
                d.rounded(theme::RADIUS_LG).border_1().border_color(if focused { theme::focus_ring() } else { theme::border() }).bg(theme::surface())
            })
            .text_size(theme::text_sm())
            .text_color(theme::text())
            .overflow_hidden()
            .when(!self.multiline, |d| d.child(TextElement { input: cx.entity() }))
            .when(self.multiline, |d| d.child(area::AreaElement { input: cx.entity() }))
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
