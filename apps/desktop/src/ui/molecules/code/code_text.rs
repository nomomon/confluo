//! One line of code in the mono font, syntax colored, never wrapped.

use gpui::{div, prelude::*, App, HighlightStyle, SharedString, StyledText, Window};

use super::lang::Lang;
use super::tokenize::{tokenize, Tok};
use crate::ui::theme;

#[derive(IntoElement)]
pub struct CodeText {
    text: SharedString,
    lang: Lang,
}

impl CodeText {
    pub fn new(text: impl Into<SharedString>, lang: Lang) -> Self {
        Self {
            text: text.into(),
            lang,
        }
    }
}

fn color(t: Tok) -> gpui::Hsla {
    match t {
        Tok::Keyword => theme::syntax_keyword(),
        Tok::String => theme::syntax_string(),
        Tok::Comment => theme::syntax_comment(),
        Tok::Number => theme::syntax_number(),
        Tok::Type => theme::syntax_type(),
    }
}

impl RenderOnce for CodeText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        // Tabs render as zero width in GPUI; expand them so indentation shows.
        let text: SharedString = if self.text.contains('\t') {
            self.text.replace('\t', "    ").into()
        } else {
            self.text
        };
        let hl: Vec<_> = tokenize(&text, self.lang)
            .into_iter()
            .map(|(r, t)| {
                (
                    r,
                    HighlightStyle {
                        color: Some(color(t)),
                        ..Default::default()
                    },
                )
            })
            .collect();
        div()
            .flex_shrink_0()
            .whitespace_nowrap()
            .font_family(theme::mono_font())
            .child(StyledText::new(text).with_highlights(hl))
    }
}
