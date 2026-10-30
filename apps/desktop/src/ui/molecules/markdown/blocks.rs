//! Parsed markdown document model.

use super::inline::Inline;

/// Column alignment of a table.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// GitHub alert kinds (`> [!NOTE]`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Alert {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl Alert {
    pub fn label(self) -> &'static str {
        match self {
            Alert::Note => "Note",
            Alert::Tip => "Tip",
            Alert::Important => "Important",
            Alert::Warning => "Warning",
            Alert::Caution => "Caution",
        }
    }
}

pub enum Block {
    Heading(u8, Inline),
    Paragraph(Inline),
    /// Fenced or indented code with its info-string language.
    Code { lang: String, code: String },
    List { start: Option<u64>, items: Vec<Item> },
    Quote { alert: Option<Alert>, blocks: Vec<Block> },
    Table { aligns: Vec<Align>, head: Vec<Inline>, rows: Vec<Vec<Inline>> },
    /// `<details><summary>..</summary>..</details>`, collapsed until clicked.
    Details { summary: String, blocks: Vec<Block> },
    Footnote { label: String, blocks: Vec<Block> },
    Rule,
}

pub struct Item {
    /// `Some(done)` for task list items.
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}
