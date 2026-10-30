//! Command palette like shadcn `Command` in a dialog: an input over a filtered list.
//! Up/Down move, Enter runs, Esc closes (the caller's Dismiss). No open animation (keyboard driven).
//!
//! `CommandPalette::new(input).items(items).selected(i).on_move(..).on_run(..).on_close(..)`

mod fuzzy;

pub use fuzzy::score;

use gpui::{actions, div, prelude::*, px, App, Entity, KeyBinding, SharedString, Window};

use super::text_input::TextInput;
use crate::ui::atoms::{Icon, IconName, Kbd};
use crate::ui::theme;

actions!(command_palette, [Up, Down]);

pub fn bind_keys(cx: &mut App) {
    let ctx = Some("CommandPalette");
    cx.bind_keys([KeyBinding::new("up", Up, ctx), KeyBinding::new("down", Down, ctx), KeyBinding::new("ctrl-p", Up, ctx), KeyBinding::new("ctrl-n", Down, ctx)]);
}

/// One row: icon, title, muted detail on the right, optional shortcut.
pub struct PaletteItem {
    pub icon: IconName,
    pub title: SharedString,
    pub detail: Option<SharedString>,
    pub shortcut: Option<SharedString>,
    /// Section heading shown above the first item of a group.
    pub group: &'static str,
}

type IndexHandler = Box<dyn Fn(&usize, &mut Window, &mut App) + 'static>;
type MoveHandler = Box<dyn Fn(&isize, &mut Window, &mut App) + 'static>;
type CloseHandler = Box<dyn Fn(&(), &mut Window, &mut App) + 'static>;

const PALETTE_W: gpui::Pixels = px(560.);
const LIST_MAX_H: gpui::Pixels = px(360.);
const MAX_ROWS: usize = 50;

#[derive(IntoElement)]
pub struct CommandPalette {
    input: Entity<TextInput>,
    items: Vec<PaletteItem>,
    selected: usize,
    on_move: Option<MoveHandler>,
    on_run: Option<IndexHandler>,
    on_close: Option<CloseHandler>,
}

impl CommandPalette {
    pub fn new(input: Entity<TextInput>) -> Self {
        Self { input, items: vec![], selected: 0, on_move: None, on_run: None, on_close: None }
    }
    pub fn items(mut self, items: Vec<PaletteItem>) -> Self {
        self.items = items;
        self
    }
    pub fn selected(mut self, i: usize) -> Self {
        self.selected = i;
        self
    }
    pub fn on_move(mut self, f: impl Fn(&isize, &mut Window, &mut App) + 'static) -> Self {
        self.on_move = Some(Box::new(f));
        self
    }
    pub fn on_run(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_run = Some(Box::new(f));
        self
    }
    pub fn on_close(mut self, f: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Box::new(f));
        self
    }
}

impl RenderOnce for CommandPalette {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let run = std::rc::Rc::new(self.on_run);
        let mv = std::rc::Rc::new(self.on_move);
        let mut list = div().id("palette-list").flex().flex_col().max_h(LIST_MAX_H).overflow_y_scroll().p(theme::SPACE_1P5);
        let mut last_group = "";
        if self.items.is_empty() {
            list = list.child(div().py(theme::SPACE_6).flex().justify_center().text_color(theme::text_subtle()).child("No results"));
        }
        for (i, it) in self.items.into_iter().take(MAX_ROWS).enumerate() {
            if it.group != last_group {
                last_group = it.group;
                list = list.child(div().px(theme::SPACE_2).pt(theme::SPACE_2).pb(theme::SPACE_1).text_size(theme::text_xs()).text_color(theme::text_subtle()).child(it.group));
            }
            let active = i == self.selected;
            let run = run.clone();
            list = list.child(
                div()
                    .id(("palette-row", i))
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_2P5)
                    .px(theme::SPACE_2)
                    .py(theme::SPACE_2)
                    .rounded(theme::RADIUS_MD)
                    .cursor_pointer()
                    .when(active, |d| d.bg(theme::surface_active()))
                    .hover(|s| s.bg(theme::surface_hover()))
                    .child(Icon::new(it.icon).color(if active { theme::text() } else { theme::text_muted() }))
                    .child(div().flex_1().min_w(theme::SPACE_0).truncate().child(it.title))
                    .children(it.detail.map(|d| div().flex_shrink_0().max_w(px(200.)).truncate().text_size(theme::text_xs()).text_color(theme::text_subtle()).child(d)))
                    .children(it.shortcut.map(Kbd::new))
                    .on_click(move |_, w, cx| {
                        if let Some(f) = run.as_ref() {
                            f(&i, w, cx)
                        }
                    }),
            );
        }
        let (up, down) = (mv.clone(), mv);
        let panel = div()
            .key_context("CommandPalette")
            .on_action(move |_: &Up, w, cx| {
                if let Some(f) = up.as_ref() {
                    f(&-1, w, cx)
                }
            })
            .on_action(move |_: &Down, w, cx| {
                if let Some(f) = down.as_ref() {
                    f(&1, w, cx)
                }
            })
            .w(PALETTE_W)
            .flex()
            .flex_col()
            .rounded(theme::RADIUS_XL)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_lg()
            .overflow_hidden()
            .child(div().p(theme::SPACE_2).border_b_1().border_color(theme::border_subtle()).child(self.input))
            .child(list);
        let close = self.on_close;
        // Instant: the palette is keyboard driven (motion rules).
        div()
            .id("palette-scrim")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(120.))
            .bg(theme::scrim())
            .occlude()
            .on_mouse_down(gpui::MouseButton::Left, move |_, w, cx| {
                if let Some(f) = close.as_ref() {
                    f(&(), w, cx)
                }
            })
            .child(div().on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation()).child(panel))
    }
}
