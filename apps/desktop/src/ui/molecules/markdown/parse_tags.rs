//! Start and end tags of the pulldown-cmark stream, applied to the `Builder`.

use pulldown_cmark::{CodeBlockKind, Tag, TagEnd};

use super::blocks::Block;
use super::inline::Inline;
use super::parse::{alert, align, Builder, Frame};

impl Builder {
    pub(super) fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.flush();
                self.inline = Some(Inline::default());
            }
            Tag::Heading { level, .. } => {
                self.flush();
                self.heading = Some(level as u8);
                self.inline = Some(Inline::default());
            }
            Tag::CodeBlock(kind) => {
                self.flush();
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or_default().to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.code = Some((lang, String::new()));
            }
            Tag::FootnoteDefinition(label) => {
                self.flush();
                self.frames.push(Frame::Footnote(label.to_string(), vec![]));
            }
            Tag::BlockQuote(kind) => {
                self.flush();
                self.frames.push(Frame::Quote(kind.map(alert), vec![]));
            }
            Tag::List(start) => {
                self.flush();
                self.frames.push(Frame::List(start, vec![]));
            }
            Tag::Item => {
                self.flush();
                self.frames.push(Frame::Item(None, vec![]));
            }
            Tag::Table(aligns) => {
                self.flush();
                self.frames.push(Frame::Table(aligns.iter().map(align).collect(), vec![], vec![]));
            }
            Tag::TableHead | Tag::TableRow => self.frames.push(Frame::Row(vec![])),
            Tag::TableCell => self.inline = Some(Inline::default()),
            Tag::Emphasis => self.marks.italic += 1,
            Tag::Strong => self.marks.bold += 1,
            Tag::Strikethrough => self.marks.strike += 1,
            Tag::Link { dest_url, .. } => self.marks.link = Some(dest_url.to_string()),
            Tag::Image { dest_url, .. } => {
                self.marks.image = Some(dest_url.to_string());
                self.marks.image_alt = false;
            }
            _ => {}
        }
    }

    pub(super) fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) => self.flush(),
            TagEnd::CodeBlock => {
                if let Some((lang, code)) = self.code.take() {
                    self.push_block(Block::Code { lang, code: code.trim_end().to_string() });
                }
            }
            TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item | TagEnd::Table | TagEnd::TableRow | TagEnd::FootnoteDefinition => self.close_frame(),
            TagEnd::TableHead => {
                self.flush();
                if let (Some(Frame::Row(cells)), Some(Frame::Table(_, head, _))) = (self.frames.pop(), self.frames.last_mut()) {
                    *head = cells;
                }
            }
            TagEnd::TableCell => {
                let cell = self.inline.take().unwrap_or_default();
                if let Some(Frame::Row(cells)) = self.frames.last_mut() {
                    cells.push(cell);
                }
            }
            TagEnd::Emphasis => self.marks.italic = self.marks.italic.saturating_sub(1),
            TagEnd::Strong => self.marks.bold = self.marks.bold.saturating_sub(1),
            TagEnd::Strikethrough => self.marks.strike = self.marks.strike.saturating_sub(1),
            TagEnd::Link => self.marks.link = None,
            TagEnd::Image => {
                if !self.marks.image_alt {
                    self.text("image", self.marks.style());
                }
                self.marks.image = None;
            }
            _ => {}
        }
    }
}
