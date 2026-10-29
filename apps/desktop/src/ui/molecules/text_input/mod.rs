//! Text input adapted from gpui's `examples/input.rs`; single-line, or multi-line
//! via `multiline()` (Enter inserts a newline, Cmd+Enter submits).

mod actions;
mod area;
mod element;
mod ime;

use gpui::{
    actions, div, prelude::*, px, App, Bounds, Context, CursorStyle, EventEmitter, FocusHandle, Focusable, KeyBinding, MouseButton, Pixels,
    ShapedLine, SharedString, Window,
};
use std::ops::Range;

use crate::ui::theme;
use element::TextElement;

actions!(
    text_input,
    [Backspace, Delete, Left, Right, SelectLeft, SelectRight, SelectAll, Home, End, Paste, Cut, Copy, Submit, Newline]
);

pub fn bind_keys(cx: &mut App) {
    let ctx = Some("TextInput");
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("left", Left, ctx),
        KeyBinding::new("right", Right, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new("cmd-a", SelectAll, ctx),
        KeyBinding::new("cmd-v", Paste, ctx),
        KeyBinding::new("cmd-c", Copy, ctx),
        KeyBinding::new("cmd-x", Cut, ctx),
        KeyBinding::new("home", Home, ctx),
        KeyBinding::new("end", End, ctx),
        KeyBinding::new("cmd-left", Home, ctx),
        KeyBinding::new("cmd-right", End, ctx),
        KeyBinding::new("enter", Submit, ctx),
        // Later bindings win at equal depth, so multi-line inputs get a newline on Enter.
        KeyBinding::new("enter", Newline, Some("TextInput && multiline")),
        KeyBinding::new("cmd-enter", Submit, ctx),
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
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    multiline: bool,
    last_paras: Vec<area::Para>,
    last_line_height: Pixels,
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
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            multiline: false,
            last_paras: vec![],
            last_line_height: px(0.),
        }
    }

    pub fn text(&self) -> String {
        self.content.to_string()
    }

    pub fn set_text(&mut self, s: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = s.into();
        let n = self.content.len();
        self.selected_range = n..n;
        self.marked_range = None;
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    /// Grow to several soft-wrapped lines; Enter inserts a newline.
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        div()
            .flex()
            .items_center()
            .key_context(if self.multiline { "TextInput multiline" } else { "TextInput" })
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::newline))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .w_full()
            .when(!self.multiline, |d| d.h(theme::CONTROL_H))
            .when(self.multiline, |d| d.items_start().min_h(theme::CONTROL_H).py(theme::SPACE_2P5))
            .px(theme::SPACE_3)
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(if focused { theme::focus_ring() } else { theme::border() })
            .bg(theme::surface())
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
