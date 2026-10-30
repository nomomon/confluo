//! Command palette like shadcn `Command` in a dialog: an input over a filtered list.
//! Up/Down move, Enter runs, Cmd+Enter runs the alternate action, Esc closes (the caller's Dismiss),
//! hovering a row highlights it. No open animation (keyboard driven).
//!
//! `CommandPalette::new(input).items(items).selected(i).footer(hint).on_move(..).on_hover(..).on_run(..).on_alt_run(..).on_close(..)`

mod fuzzy;
mod reveal;

pub use fuzzy::score;

use gpui::{actions, div, prelude::*, px, App, Entity, KeyBinding, SharedString, Window};

use super::text_input::TextInput;
use crate::ui::atoms::{Icon, IconName, Kbd, ListItem};
use crate::ui::theme;

actions!(command_palette, [Up, Down, AltRun]);

pub fn bind_keys(cx: &mut App) {
    let ctx = Some("CommandPalette");
    cx.bind_keys([
        KeyBinding::new("up", Up, ctx),
        KeyBinding::new("down", Down, ctx),
        KeyBinding::new("ctrl-p", Up, ctx),
        KeyBinding::new("ctrl-n", Down, ctx),
    ]);
    // Deeper than the text input's own Cmd+Enter (Submit), registered after it, so it wins here.
    cx.bind_keys([KeyBinding::new(
        "cmd-enter",
        AltRun,
        Some("CommandPalette > TextInput"),
    )]);
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
    footer: Option<SharedString>,
    on_move: Option<MoveHandler>,
    on_hover: Option<IndexHandler>,
    on_run: Option<IndexHandler>,
    on_alt_run: Option<IndexHandler>,
    on_close: Option<CloseHandler>,
}

impl CommandPalette {
    pub fn new(input: Entity<TextInput>) -> Self {
        Self {
            input,
            items: vec![],
            selected: 0,
            footer: None,
            on_move: None,
            on_hover: None,
            on_run: None,
            on_alt_run: None,
            on_close: None,
        }
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
    /// Muted hint line under the list (e.g. the query prefixes).
    pub fn footer(mut self, text: impl Into<SharedString>) -> Self {
        self.footer = Some(text.into());
        self
    }
    /// The pointer entered row `i`.
    pub fn on_hover(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Box::new(f));
        self
    }
    /// Cmd+Enter on the selected row.
    pub fn on_alt_run(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_alt_run = Some(Box::new(f));
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
        let hover = std::rc::Rc::new(self.on_hover);
        let (alt, selected) = (self.on_alt_run, self.selected);
        // Rows are direct children of the scroller so its handle can reveal them; each is shifted by
        // the rubber band like `elastic::content`.
        let shift = crate::ui::elastic::shift("palette-list");
        let mut list: Vec<gpui::AnyElement> = vec![];
        let rows: Vec<_> = self.items.into_iter().take(MAX_ROWS).collect();
        let groups: Vec<&str> = rows.iter().map(|r| r.group).collect();
        let targets = reveal::targets(&groups);
        let n = rows.len();
        let mut last_group = "";
        if rows.is_empty() {
            list.push(
                div()
                    .py(theme::SPACE_6)
                    .flex()
                    .justify_center()
                    .text_color(theme::text_subtle())
                    .child("No results")
                    .into_any_element(),
            );
        }
        for (i, it) in rows.into_iter().enumerate() {
            if it.group != last_group {
                last_group = it.group;
                list.push(
                    div()
                        .relative()
                        .top(shift)
                        .px(theme::SPACE_2)
                        .pt(theme::SPACE_2)
                        .pb(theme::SPACE_1)
                        .text_size(theme::text_xs())
                        .text_color(theme::text_subtle())
                        .child(it.group)
                        .into_any_element(),
                );
            }
            let active = i == self.selected;
            let run = run.clone();
            let hover = hover.clone();
            list.push(
                div()
                    .id(("palette-row", i))
                    .relative()
                    .top(shift)
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_2P5)
                    .px(theme::SPACE_2)
                    .py(theme::item_pad_y())
                    .list_item(active)
                    .child(Icon::new(it.icon).color(if active {
                        theme::text()
                    } else {
                        theme::text_muted()
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme::SPACE_0)
                            .truncate()
                            .child(it.title),
                    )
                    .children(it.detail.map(|d| {
                        div()
                            .flex_shrink_0()
                            .max_w(px(200.))
                            .truncate()
                            .text_size(theme::text_xs())
                            .text_color(theme::text_subtle())
                            .child(d)
                    }))
                    .children(it.shortcut.map(Kbd::new))
                    .on_hover(move |on, w, cx| {
                        if let (true, Some(f)) = (*on, hover.as_ref()) {
                            f(&i, w, cx)
                        }
                    })
                    .on_click(move |_, w, cx| {
                        if let Some(f) = run.as_ref() {
                            f(&i, w, cx)
                        }
                    })
                    .into_any_element(),
            );
        }
        let h = crate::ui::elastic::handle("palette-list");
        // Keyboard moves reveal the new row (minimal scroll); the caller wraps the same way.
        let rh = h.clone();
        let reveal = std::rc::Rc::new(move |d: isize| {
            if let Some(&t) = targets.get(reveal::step(selected, d, n)) {
                rh.scroll_to_item(t);
            }
        });
        let (rv_up, rv_down) = (reveal.clone(), reveal);
        let (up, down) = (mv.clone(), mv);
        let panel = div()
            .key_context("CommandPalette")
            .on_action(move |_: &Up, w, cx| {
                rv_up(-1);
                if let Some(f) = up.as_ref() {
                    f(&-1, w, cx)
                }
            })
            .on_action(move |_: &Down, w, cx| {
                rv_down(1);
                if let Some(f) = down.as_ref() {
                    f(&1, w, cx)
                }
            })
            .on_action(move |_: &AltRun, w, cx| {
                if let Some(f) = alt.as_ref() {
                    f(&selected, w, cx)
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
            .child(
                div()
                    .p(theme::SPACE_2)
                    .border_b_1()
                    .border_color(theme::border_subtle())
                    .child(self.input),
            )
            .child({
                let pane = div()
                    .id("palette-list")
                    .track_scroll(&h)
                    .max_h(LIST_MAX_H)
                    .map(crate::ui::scroll::y_only)
                    .p(theme::SPACE_1P5)
                    .flex()
                    .flex_col()
                    .children(list);
                crate::ui::elastic::y_scroller(pane, "palette-list", h)
            })
            .children(self.footer.map(|f| {
                div()
                    .px(theme::SPACE_3)
                    .py(theme::SPACE_2)
                    .border_t_1()
                    .border_color(theme::border_subtle())
                    .text_size(theme::text_xs())
                    .text_color(theme::text_subtle())
                    .child(f)
            }));
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
            .child(
                div()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(panel),
            )
    }
}
