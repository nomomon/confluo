//! Code view tokens: diff line fills and syntax highlight colors, per theme.

use gpui::Hsla;

use super::palette::syntax;
use super::runtime::is_light;
use super::tokens::{danger, success, surface, text_subtle};

fn pick(dark: Hsla, light: Hsla) -> Hsla {
    if is_light() {
        light
    } else {
        dark
    }
}

/// Background of an added line.
pub fn diff_add_bg() -> Hsla {
    success().opacity(if is_light() { 0.10 } else { 0.08 })
}
/// Background of a removed line.
pub fn diff_del_bg() -> Hsla {
    danger().opacity(if is_light() { 0.10 } else { 0.08 })
}
/// The subtle band behind `@@` hunk headers.
pub fn diff_hunk_bg() -> Hsla {
    surface()
}
pub fn syntax_keyword() -> Hsla {
    pick(syntax::KEYWORD_DARK, syntax::KEYWORD_LIGHT)
}
pub fn syntax_string() -> Hsla {
    pick(syntax::STRING_DARK, syntax::STRING_LIGHT)
}
pub fn syntax_number() -> Hsla {
    pick(syntax::NUMBER_DARK, syntax::NUMBER_LIGHT)
}
pub fn syntax_type() -> Hsla {
    pick(syntax::TYPE_DARK, syntax::TYPE_LIGHT)
}
pub fn syntax_comment() -> Hsla {
    text_subtle()
}
