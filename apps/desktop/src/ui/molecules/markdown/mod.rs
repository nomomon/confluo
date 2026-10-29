//! Markdown for PR descriptions and comments, parsed with pulldown-cmark.
//!
//! `markdown("desc", src)` returns block elements: headings, paragraphs with
//! emphasis, inline code and clickable links, code blocks, lists and task lists,
//! quotes (GitHub alerts too), tables as a grid and rules. Images show only their
//! alt text; HTML comments are dropped and `<details>` collapse to their summary.

mod blocks;
mod html;
mod inline;
mod parse;
mod render;

use gpui::{AnyElement, SharedString};

pub fn markdown(id: impl Into<SharedString>, src: &str) -> Vec<AnyElement> {
    let blocks = parse::parse(&html::preprocess(src));
    render::render(&id.into(), &blocks)
}
