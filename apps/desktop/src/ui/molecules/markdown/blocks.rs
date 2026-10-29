//! Parsed markdown document model.

use super::inline::Inline;

pub enum Block {
    Heading(u8, Inline),
    Paragraph(Inline),
    Code(String),
    List { start: Option<u64>, items: Vec<Item> },
    Quote { alert: Option<&'static str>, blocks: Vec<Block> },
    Table { head: Vec<Inline>, rows: Vec<Vec<Inline>> },
    Rule,
}

pub struct Item {
    /// `Some(done)` for task list items.
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}
