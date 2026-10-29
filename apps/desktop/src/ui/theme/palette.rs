//! Raw color scales (Tailwind-inspired). Prefer the semantic tokens in `tokens.rs`.

// Full scales are part of the design-system API even where unused.
#![allow(dead_code)]

use gpui::Hsla;

const fn c(h: f32, s: f32, l: f32) -> Hsla {
    Hsla { h: h / 360., s, l, a: 1. }
}

/// Neutral zinc-like scale, 50 (lightest) to 950 (darkest).
pub mod zinc {
    use super::{c, Hsla};
    pub const Z50: Hsla = c(0., 0., 0.98);
    pub const Z100: Hsla = c(240., 0.048, 0.959);
    pub const Z200: Hsla = c(240., 0.059, 0.90);
    pub const Z300: Hsla = c(240., 0.049, 0.839);
    pub const Z400: Hsla = c(240., 0.05, 0.649);
    pub const Z500: Hsla = c(240., 0.038, 0.461);
    pub const Z600: Hsla = c(240., 0.052, 0.339);
    pub const Z700: Hsla = c(240., 0.053, 0.261);
    pub const Z800: Hsla = c(240., 0.037, 0.159);
    pub const Z900: Hsla = c(240., 0.059, 0.10);
    pub const Z950: Hsla = c(240., 0.10, 0.039);
}

/// Accents, used only where color carries meaning. Tuned 400 shades for dark UIs.
pub const GREEN: Hsla = c(142., 0.60, 0.56);
pub const PURPLE: Hsla = c(270., 0.80, 0.74);
pub const RED: Hsla = c(0., 0.85, 0.68);
pub const AMBER: Hsla = c(43., 0.92, 0.58);
pub const BLUE: Hsla = c(213., 0.90, 0.68);
pub const WHITE: Hsla = c(0., 0., 1.);
pub const BLACK: Hsla = c(0., 0., 0.);
