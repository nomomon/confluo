//! Multi-line text field like shadcn `Textarea`: a `TextInput::multiline()` that grows from
//! `rows` to `max_rows` visible lines, then scrolls inside. The input draws the only border.
//!
//! `Textarea::new(composer.clone()).rows(4).max_rows(16)`

use gpui::{div, prelude::*, App, Entity, Window};

use super::text_input::TextInput;

#[derive(IntoElement)]
pub struct Textarea {
    input: Entity<TextInput>,
    rows: u8,
    max_rows: u8,
}

impl Textarea {
    pub fn new(input: Entity<TextInput>) -> Self {
        Self {
            input,
            rows: 3,
            max_rows: 12,
        }
    }

    pub fn rows(mut self, rows: u8) -> Self {
        self.rows = rows;
        self
    }

    pub fn max_rows(mut self, rows: u8) -> Self {
        self.max_rows = rows;
        self
    }
}

impl RenderOnce for Textarea {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (min, max) = (usize::from(self.rows), usize::from(self.max_rows));
        self.input.update(cx, |i, _| i.set_rows(min, max));
        div().w_full().flex().flex_col().child(self.input)
    }
}
