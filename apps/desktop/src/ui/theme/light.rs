//! Light theme surfaces. Candidates A/B/C are compared with `CONFLUO_LIGHT_VARIANT` (dev only).
//!
//! The window background is the frame; content regions paint an almost opaque fill over it.

use std::sync::atomic::{AtomicU8, Ordering};

use gpui::Hsla;

const fn g(h: f32, s: f32, l: f32) -> Hsla {
    Hsla {
        h: h / 360.,
        s,
        l,
        a: 1.,
    }
}

const WHITE: Hsla = g(0., 0., 1.);

/// Surface roles in light mode.
pub(super) struct LightSurfaces {
    /// Window frame (behind everything).
    pub frame: Hsla,
    /// Minimum frame alpha over the desktop.
    pub frame_min: f32,
    /// Sidebar fill and its minimum alpha.
    pub sidebar: Hsla,
    pub sidebar_min: f32,
    /// Sidebar floats as an inset rounded panel (macOS 26).
    pub sidebar_inset: bool,
    /// List and diff column.
    pub content: Hsla,
    /// Preview and Review side panes.
    pub panel: Hsla,
    /// Cards and list sections.
    pub card: Hsla,
    /// Card outline alpha (black); 0 for flat sections.
    pub card_border: f32,
}

/// A: gray frame and sidebar (#F2F2F4), white content, white cards with hairlines.
const A: LightSurfaces = LightSurfaces {
    frame: g(240., 0.08, 0.953),
    frame_min: 0.94,
    sidebar: g(240., 0.08, 0.953),
    sidebar_min: 0.8,
    sidebar_inset: false,
    content: WHITE,
    panel: WHITE,
    card: WHITE,
    card_border: 0.08,
};
/// B: white frame, #F5F5F7 sidebar, borderless #F7F7F8 cards.
const B: LightSurfaces = LightSurfaces {
    frame: WHITE,
    frame_min: 0.94,
    sidebar: g(240., 0.11, 0.963),
    sidebar_min: 0.92,
    sidebar_inset: false,
    content: WHITE,
    panel: WHITE,
    card: g(240., 0.08, 0.969),
    card_border: 0.,
};
/// C: macOS 26 materials. White flat content, floating inset glass sidebar, sections split by hairlines only.
const C: LightSurfaces = LightSurfaces {
    frame: WHITE,
    frame_min: 0.94,
    sidebar: g(240., 0.11, 0.963),
    sidebar_min: 0.86,
    sidebar_inset: true,
    content: WHITE,
    panel: WHITE,
    card: WHITE,
    card_border: 0.,
};

static VARIANT: AtomicU8 = AtomicU8::new(b'c');

/// Pick a light candidate (`a`, `b` or `c`); anything else keeps the default.
pub fn set_light_variant(v: &str) {
    if let Some(&c) = v
        .as_bytes()
        .first()
        .filter(|c| matches!(c, b'a' | b'b' | b'c'))
    {
        VARIANT.store(c, Ordering::Relaxed);
    }
}

pub(super) fn surfaces() -> &'static LightSurfaces {
    match VARIANT.load(Ordering::Relaxed) {
        b'a' => &A,
        b'b' => &B,
        _ => &C,
    }
}

/// Whether the sidebar floats as an inset panel (light C only).
pub fn sidebar_inset() -> bool {
    super::is_light() && surfaces().sidebar_inset
}
