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

/// Accents for dark UIs (light 400-ish shades), used only where color carries meaning.
pub mod dark {
    use super::{c, Hsla};
    pub const GREEN: Hsla = c(142., 0.60, 0.56);
    pub const PURPLE: Hsla = c(270., 0.80, 0.74);
    pub const RED: Hsla = c(0., 0.85, 0.68);
    pub const AMBER: Hsla = c(43., 0.92, 0.58);
    pub const BLUE: Hsla = c(213., 0.90, 0.68);
}

/// Light UI: soft grey page and darker accents for contrast on it.
pub mod light {
    use super::{c, Hsla};
    /// Page background, about #f3f3f5.
    pub const PAGE: Hsla = c(240., 0.11, 0.955);
    /// Panel fill (sidebar, detail), a touch lighter than the page.
    pub const PANEL: Hsla = c(240., 0.12, 0.975);
    /// Card fill, slightly darker than the page (about #e9e9ec).
    pub const CARD: Hsla = c(240., 0.08, 0.885);
    pub const GREEN: Hsla = c(142., 0.62, 0.32);
    pub const PURPLE: Hsla = c(270., 0.55, 0.50);
    pub const RED: Hsla = c(0., 0.70, 0.45);
    pub const AMBER: Hsla = c(35., 0.92, 0.38);
    pub const BLUE: Hsla = c(213., 0.80, 0.46);
}
pub const WHITE: Hsla = c(0., 0., 1.);
pub const BLACK: Hsla = c(0., 0., 0.);

/// Syntax highlighting hues for code views, dark and light variants.
pub mod syntax {
    use super::{c, Hsla};
    pub const KEYWORD_DARK: Hsla = c(330., 0.75, 0.72);
    pub const KEYWORD_LIGHT: Hsla = c(330., 0.65, 0.42);
    pub const STRING_DARK: Hsla = c(160., 0.55, 0.62);
    pub const STRING_LIGHT: Hsla = c(160., 0.70, 0.28);
    pub const NUMBER_DARK: Hsla = c(28., 0.85, 0.66);
    pub const NUMBER_LIGHT: Hsla = c(28., 0.85, 0.38);
    pub const TYPE_DARK: Hsla = c(195., 0.70, 0.68);
    pub const TYPE_LIGHT: Hsla = c(205., 0.75, 0.38);
}
