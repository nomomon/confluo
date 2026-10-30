//! pulldown-cmark events to `Block`s.

use pulldown_cmark::{Alignment, BlockQuoteKind, Event, Options, Parser};

use super::blocks::{Alert, Align, Block, Item};
use super::html::{DETAILS_CLOSE, DETAILS_OPEN};
use super::inline::{Inline, Style};

pub(super) enum Frame {
    Root(Vec<Block>),
    Quote(Option<Alert>, Vec<Block>),
    List(Option<u64>, Vec<Item>),
    Item(Option<bool>, Vec<Block>),
    Table(Vec<Align>, Vec<Inline>, Vec<Vec<Inline>>),
    Row(Vec<Inline>),
    Details(String, Vec<Block>),
    Footnote(String, Vec<Block>),
}

#[derive(Default)]
pub(super) struct Marks {
    pub(super) bold: u32,
    pub(super) italic: u32,
    pub(super) strike: u32,
    pub(super) image: Option<String>,
    /// Alt text seen inside the current image.
    pub(super) image_alt: bool,
    pub(super) link: Option<String>,
}

impl Marks {
    pub(super) fn style(&self) -> Style {
        Style { bold: self.bold > 0, italic: self.italic > 0, strike: self.strike > 0, code: false, image: self.image.clone(), link: self.link.clone() }
    }
}

pub(super) struct Builder {
    pub(super) frames: Vec<Frame>,
    pub(super) inline: Option<Inline>,
    pub(super) heading: Option<u8>,
    pub(super) code: Option<(String, String)>,
    pub(super) marks: Marks,
}

pub fn parse(src: &str) -> Vec<Block> {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_GFM | Options::ENABLE_FOOTNOTES;
    let mut b = Builder { frames: vec![Frame::Root(vec![])], inline: None, heading: None, code: None, marks: Marks::default() };
    for ev in Parser::new_ext(src, opts) {
        b.event(ev);
    }
    b.flush();
    while b.frames.len() > 1 {
        b.close_frame();
    }
    match b.frames.pop() {
        Some(Frame::Root(blocks)) => blocks,
        _ => vec![],
    }
}

pub(super) fn alert(kind: BlockQuoteKind) -> Alert {
    match kind {
        BlockQuoteKind::Note => Alert::Note,
        BlockQuoteKind::Tip => Alert::Tip,
        BlockQuoteKind::Important => Alert::Important,
        BlockQuoteKind::Warning => Alert::Warning,
        BlockQuoteKind::Caution => Alert::Caution,
    }
}

pub(super) fn align(a: &Alignment) -> Align {
    match a {
        Alignment::Center => Align::Center,
        Alignment::Right => Align::Right,
        _ => Align::Left,
    }
}

impl Builder {
    pub(super) fn push_block(&mut self, block: Block) {
        match self.frames.last_mut() {
            Some(Frame::Root(v) | Frame::Quote(_, v) | Frame::Item(_, v) | Frame::Details(_, v) | Frame::Footnote(_, v)) => v.push(block),
            // A block directly in a list (should not happen): wrap it in an item.
            Some(Frame::List(_, items)) => items.push(Item { task: None, blocks: vec![block] }),
            _ => {}
        }
    }

    /// Close the pending inline text as a paragraph or heading.
    pub(super) fn flush(&mut self) {
        let Some(inline) = self.inline.take() else { return };
        let level = self.heading.take();
        if inline.is_blank() {
            return;
        }
        if level.is_none() {
            if let Some(summary) = inline.text.trim().strip_prefix(DETAILS_OPEN) {
                self.frames.push(Frame::Details(summary.trim().to_string(), vec![]));
                return;
            }
            if inline.text.trim() == DETAILS_CLOSE {
                if self.frames.iter().any(|f| matches!(f, Frame::Details(..))) {
                    while !matches!(self.frames.last(), Some(Frame::Details(..))) {
                        self.close_frame();
                    }
                    self.close_frame();
                }
                return;
            }
        }
        self.push_block(match level {
            Some(l) => Block::Heading(l, inline),
            None => Block::Paragraph(inline),
        });
    }

    pub(super) fn close_frame(&mut self) {
        self.flush();
        let Some(frame) = self.frames.pop() else { return };
        match frame {
            Frame::Quote(alert, blocks) => self.push_block(Block::Quote { alert, blocks }),
            Frame::List(start, items) => self.push_block(Block::List { start, items }),
            Frame::Item(task, blocks) => {
                if let Some(Frame::List(_, items)) = self.frames.last_mut() {
                    items.push(Item { task, blocks });
                }
            }
            Frame::Table(aligns, head, rows) => self.push_block(Block::Table { aligns, head, rows }),
            Frame::Details(summary, blocks) => self.push_block(Block::Details { summary, blocks }),
            Frame::Footnote(label, blocks) => self.push_block(Block::Footnote { label, blocks }),
            Frame::Row(cells) => {
                if let Some(Frame::Table(_, _, rows)) = self.frames.last_mut() {
                    rows.push(cells);
                }
            }
            Frame::Root(_) => {}
        }
    }

    pub(super) fn text(&mut self, s: &str, style: Style) {
        if style.image.is_some() {
            self.marks.image_alt = true;
        }
        self.inline.get_or_insert_with(Inline::default).push_autolinked(s, style);
    }

    fn event(&mut self, ev: Event) {
        match ev {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(t) => match self.code.as_mut() {
                Some((_, code)) => code.push_str(&t),
                None => self.text(&t, self.marks.style()),
            },
            Event::Code(t) => self.text(&t, Style { code: true, ..self.marks.style() }),
            Event::FootnoteReference(label) => self.text(&format!("[{label}]"), Style { bold: true, ..self.marks.style() }),
            Event::InlineHtml(h) if h.trim().eq_ignore_ascii_case("<br>") || h.trim().eq_ignore_ascii_case("<br/>") => self.text("\n", self.marks.style()),
            Event::SoftBreak => self.text(" ", self.marks.style()),
            Event::HardBreak => self.text("\n", self.marks.style()),
            Event::Rule => {
                self.flush();
                self.push_block(Block::Rule);
            }
            Event::TaskListMarker(done) => {
                if let Some(Frame::Item(task, _)) = self.frames.last_mut() {
                    *task = Some(done);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
