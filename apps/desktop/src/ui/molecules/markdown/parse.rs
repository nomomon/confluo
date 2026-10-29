//! pulldown-cmark events to `Block`s.

use pulldown_cmark::{BlockQuoteKind, Event, Options, Parser, Tag, TagEnd};

use super::blocks::{Block, Item};
use super::inline::{Inline, Style};

enum Frame {
    Root(Vec<Block>),
    Quote(Option<&'static str>, Vec<Block>),
    List(Option<u64>, Vec<Item>),
    Item(Option<bool>, Vec<Block>),
    Table(Vec<Inline>, Vec<Vec<Inline>>),
    Row(Vec<Inline>),
}

#[derive(Default)]
struct Marks {
    bold: u32,
    italic: u32,
    strike: u32,
    image: u32,
    link: Option<String>,
}

impl Marks {
    fn style(&self) -> Style {
        Style { bold: self.bold > 0, italic: self.italic > 0, strike: self.strike > 0, code: false, muted: self.image > 0, link: self.link.clone() }
    }
}

struct Builder {
    frames: Vec<Frame>,
    inline: Option<Inline>,
    heading: Option<u8>,
    code: Option<String>,
    marks: Marks,
}

pub fn parse(src: &str) -> Vec<Block> {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_GFM;
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

fn alert(kind: BlockQuoteKind) -> &'static str {
    match kind {
        BlockQuoteKind::Note => "Note",
        BlockQuoteKind::Tip => "Tip",
        BlockQuoteKind::Important => "Important",
        BlockQuoteKind::Warning => "Warning",
        BlockQuoteKind::Caution => "Caution",
    }
}

impl Builder {
    fn push_block(&mut self, block: Block) {
        match self.frames.last_mut() {
            Some(Frame::Root(v) | Frame::Quote(_, v) | Frame::Item(_, v)) => v.push(block),
            // A block directly in a list (should not happen): wrap it in an item.
            Some(Frame::List(_, items)) => items.push(Item { task: None, blocks: vec![block] }),
            _ => {}
        }
    }

    /// Close the pending inline text as a paragraph or heading.
    fn flush(&mut self) {
        let Some(inline) = self.inline.take() else { return };
        let level = self.heading.take();
        if inline.is_blank() {
            return;
        }
        self.push_block(match level {
            Some(l) => Block::Heading(l, inline),
            None => Block::Paragraph(inline),
        });
    }

    fn close_frame(&mut self) {
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
            Frame::Table(head, rows) => self.push_block(Block::Table { head, rows }),
            Frame::Row(cells) => {
                if let Some(Frame::Table(_, rows)) = self.frames.last_mut() {
                    rows.push(cells);
                }
            }
            Frame::Root(_) => {}
        }
    }

    fn text(&mut self, s: &str, style: Style) {
        self.inline.get_or_insert_with(Inline::default).push(s, style);
    }

    fn event(&mut self, ev: Event) {
        match ev {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(t) => match self.code.as_mut() {
                Some(code) => code.push_str(&t),
                None => self.text(&t, self.marks.style()),
            },
            Event::Code(t) => self.text(&t, Style { code: true, ..self.marks.style() }),
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

    fn start(&mut self, tag: Tag) {
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
            Tag::CodeBlock(_) => {
                self.flush();
                self.code = Some(String::new());
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
            Tag::Table(_) => {
                self.flush();
                self.frames.push(Frame::Table(vec![], vec![]));
            }
            Tag::TableHead | Tag::TableRow => self.frames.push(Frame::Row(vec![])),
            Tag::TableCell => self.inline = Some(Inline::default()),
            Tag::Emphasis => self.marks.italic += 1,
            Tag::Strong => self.marks.bold += 1,
            Tag::Strikethrough => self.marks.strike += 1,
            Tag::Link { dest_url, .. } => self.marks.link = Some(dest_url.to_string()),
            Tag::Image { .. } => self.marks.image += 1,
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) => self.flush(),
            TagEnd::CodeBlock => {
                if let Some(code) = self.code.take() {
                    self.push_block(Block::Code(code.trim_end().to_string()));
                }
            }
            TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item | TagEnd::Table | TagEnd::TableRow => self.close_frame(),
            TagEnd::TableHead => {
                self.flush();
                if let (Some(Frame::Row(cells)), Some(Frame::Table(head, _))) = (self.frames.pop(), self.frames.last_mut()) {
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
            TagEnd::Image => self.marks.image = self.marks.image.saturating_sub(1),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::ui::molecules::markdown::blocks::Block;

    #[test]
    fn table_links_and_images() {
        let src = "| Package | Age |\n|---|---|\n| [react](https://x.dev) | ![age](https://img) |\n";
        let blocks = parse(src);
        let Some(Block::Table { head, rows }) = blocks.first() else { panic!("expected a table") };
        assert_eq!(head.len(), 2);
        assert_eq!(rows[0][0].text, "react");
        assert_eq!(rows[0][0].links()[0].1, "https://x.dev");
        assert_eq!(rows[0][1].text, "age");
        assert!(rows[0][1].runs[0].1.muted);
    }

    #[test]
    fn tight_task_list() {
        let blocks = parse("- [x] done\n- [ ] todo\n");
        let Some(Block::List { items, .. }) = blocks.first() else { panic!("expected a list") };
        assert_eq!(items[0].task, Some(true));
        assert_eq!(items[1].task, Some(false));
        assert!(matches!(items[0].blocks[0], Block::Paragraph(_)));
    }

    #[test]
    fn gfm_alert() {
        let blocks = parse("> [!WARNING]\n> Careful\n");
        assert!(matches!(blocks.first(), Some(Block::Quote { alert: Some("Warning"), .. })));
    }
}
