//! Multi-line text field like shadcn `Textarea`: a `TextInput::multiline()` with a minimum height.
//!
//! `Textarea::new(composer.clone()).rows(4)`

use gpui::{div, prelude::*, App, Entity, Window};

use super::text_input::TextInput;
use crate::ui::theme;

#[derive(IntoElement)]
pub struct Textarea {
    input: Entity<TextInput>,
    rows: u8,
}

impl Textarea {
    pub fn new(input: Entity<TextInput>) -> Self {
        Self { input, rows: 3 }
    }

    pub fn rows(mut self, rows: u8) -> Self {
        self.rows = rows;
        self
    }
}

impl RenderOnce for Textarea {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let line = theme::text_sm() * 1.5;
        div().w_full().min_h(line * f32::from(self.rows) + theme::SPACE_5).flex().flex_col().child(self.input)
    }
}
