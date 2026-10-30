//! Markdown for PR descriptions and comments, parsed with pulldown-cmark.
//!
//! `Markdown::new("desc", src).images(loaded)` renders headings, paragraphs with emphasis, inline
//! code and links (hover underline), fenced code with syntax colors and a copy button, nested and
//! task lists, quotes and GitHub alerts as tinted callouts, tables (header, borders, alignment,
//! horizontal scroll), collapsible `<details>`, remote images (from `images`, else alt text),
//! autolinks, strikethrough and footnotes. HTML comments are dropped.

mod blocks;
mod html;
mod inline;
mod parse;
mod parse_tags;
mod render;
mod table;
mod text;
mod widgets;

use gpui::{div, prelude::*, App, Image, SharedString, Window};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Loaded images by URL; missing ones render as their alt text.
pub type Images = Rc<HashMap<String, Arc<Image>>>;

#[derive(IntoElement)]
pub struct Markdown {
    id: SharedString,
    src: String,
    images: Images,
}

impl Markdown {
    pub fn new(id: impl Into<SharedString>, src: &str) -> Self {
        Self {
            id: id.into(),
            src: src.to_string(),
            images: Rc::default(),
        }
    }

    pub fn images(mut self, images: Images) -> Self {
        self.images = images;
        self
    }
}

impl RenderOnce for Markdown {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let blocks = parse::parse(&html::preprocess(&self.src));
        div()
            .flex()
            .flex_col()
            .min_w(crate::ui::theme::SPACE_0)
            .children(render::render(&self.id, &blocks, self.images))
    }
}

/// Remote image URLs in `src`, so the caller can fetch them.
pub fn image_urls(src: &str) -> Vec<String> {
    let mut out = vec![];
    for ev in pulldown_cmark::Parser::new_ext(src, pulldown_cmark::Options::ENABLE_TABLES) {
        if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { dest_url, .. }) = ev {
            if dest_url.starts_with("https://") || dest_url.starts_with("http://") {
                out.push(dest_url.to_string());
            }
        }
    }
    out
}
