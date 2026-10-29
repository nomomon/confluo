//! Code display: a lightweight per-language tokenizer and diff/file line rows.

mod code_text;
mod diff_row;
mod lang;
mod tokenize;

pub use code_text::CodeText;
pub use diff_row::{DiffRow, LineMark};
pub use lang::Lang;
