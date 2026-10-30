//! The one open floating layer (menu, popover, combobox, picker) as the window sees it this frame.
//!
//! Every floating panel calls `track` so its painted bounds are recorded; the root calls
//! `begin_frame` before rendering children. Tooltips and hover cards ask `blocks(mouse)` when they
//! are about to show: while a layer is open, only targets inside it may show one, so a hover card
//! never appears next to an open popover.

use gpui::{canvas, prelude::*, Bounds, Pixels, Point};
use std::cell::Cell;

thread_local! {
    static OPEN: Cell<Option<Bounds<Pixels>>> = const { Cell::new(None) };
}

/// Forget last frame's layer; call from the root before children render.
pub fn begin_frame() {
    OPEN.with(|o| o.set(None));
}

/// True when a layer is open and `pos` lies outside it: tooltips there must stay hidden.
pub fn blocks(pos: Point<Pixels>) -> bool {
    OPEN.with(|o| o.get().is_some_and(|b| !b.contains(&pos)))
}

fn mark(b: Bounds<Pixels>) {
    OPEN.with(|o| o.set(Some(o.get().map_or(b, |a| a.union(&b)))));
}

/// Record the panel's painted bounds as the open layer (adds an invisible full-size child; makes it `relative`).
pub fn track<E: ParentElement + Styled>(panel: E) -> E {
    panel.relative().child(
        canvas(|b, _, _| mark(b), |_, _, _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
    )
}
