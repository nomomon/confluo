//! `Block`s to GPUI elements, styled only from `ui::theme`.

use gpui::{div, font, prelude::*, AnyElement, FontStyle, FontWeight, InteractiveText, SharedString, StrikethroughStyle, StyledText, TextRun, UnderlineStyle};
use std::cell::Cell;

use super::blocks::{Block, Item};
use super::inline::Inline;
use crate::ui::atoms::{Icon, IconName};
use crate::ui::theme;

/// Hands out unique element ids for clickable text within one document.
struct Ids<'a> {
    prefix: &'a str,
    next: Cell<usize>,
}

impl Ids<'_> {
    fn next(&self) -> SharedString {
        let n = self.next.get();
        self.next.set(n + 1);
        format!("{}-md-{n}", self.prefix).into()
    }
}

pub fn render(prefix: &str, blocks: &[Block]) -> Vec<AnyElement> {
    let ids = Ids { prefix, next: Cell::new(0) };
    blocks.iter().map(|b| block(&ids, b)).collect()
}

fn block(ids: &Ids, b: &Block) -> AnyElement {
    match b {
        Block::Heading(level, text) => {
            let size = match level {
                1 => theme::text_lg(),
                2 => theme::text_base(),
                _ => theme::text_sm(),
            };
            div().pt(theme::SPACE_1).text_size(size).child(inline(ids, text, theme::WEIGHT_SEMIBOLD)).into_any_element()
        }
        Block::Paragraph(text) => div().text_size(theme::text_sm()).child(inline(ids, text, theme::WEIGHT_NORMAL)).into_any_element(),
        Block::Code(code) => div()
            .p(theme::SPACE_2)
            .rounded(theme::RADIUS_MD)
            .bg(theme::surface())
            .text_size(theme::text_xs())
            .font_family(theme::mono_font())
            .overflow_hidden()
            .child(code.clone())
            .into_any_element(),
        Block::List { start, items } => div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_1)
            .children(items.iter().enumerate().map(|(i, item)| list_item(ids, *start, i, item)))
            .into_any_element(),
        Block::Quote { alert, blocks } => div()
            .flex()
            .flex_col()
            .gap(theme::SPACE_1)
            .pl(theme::SPACE_3)
            .border_l_2()
            .border_color(if alert.is_some() { theme::warning() } else { theme::border() })
            .text_color(theme::text_muted())
            .when_some(*alert, |d, a| d.child(div().text_size(theme::text_xs()).font_weight(theme::WEIGHT_SEMIBOLD).text_color(theme::warning()).child(a)))
            .children(blocks.iter().map(|b| block(ids, b)))
            .into_any_element(),
        Block::Table { head, rows } => table(ids, head, rows),
        Block::Rule => div().my(theme::SPACE_1).h(gpui::px(1.)).bg(theme::border_subtle()).into_any_element(),
    }
}

fn list_item(ids: &Ids, start: Option<u64>, i: usize, item: &Item) -> AnyElement {
    let marker = match (item.task, start) {
        (Some(done), _) => div()
            .mt(theme::SPACE_0P5)
            .flex()
            .items_center()
            .justify_center()
            .size(theme::ICON_MD)
            .rounded(theme::RADIUS_SM)
            .border_1()
            .border_color(theme::border())
            .when(done, |d| d.bg(theme::tint_bg(theme::success())).child(Icon::new(IconName::Check).size(theme::ICON_SM).color(theme::success())))
            .into_any_element(),
        (None, Some(n)) => div().text_color(theme::text_muted()).child(format!("{}.", n + i as u64)).into_any_element(),
        (None, None) => div().text_color(theme::text_subtle()).child("\u{2022}").into_any_element(),
    };
    div()
        .flex()
        .gap(theme::SPACE_2)
        .text_size(theme::text_sm())
        .child(div().flex().justify_end().w(theme::SPACE_4).flex_shrink_0().child(marker))
        .child(div().flex().flex_col().flex_1().min_w(theme::SPACE_0).gap(theme::SPACE_1).children(item.blocks.iter().map(|b| block(ids, b))))
        .into_any_element()
}

fn table(ids: &Ids, head: &[Inline], rows: &[Vec<Inline>]) -> AnyElement {
    let cols = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
    let row = |cells: &[Inline], is_head: bool| {
        div()
            .flex()
            .border_b_1()
            .border_color(theme::border_subtle())
            .when(is_head, |d| d.bg(theme::surface()).text_color(theme::text_muted()))
            .children((0..cols).map(|i| {
                let cell = div().flex_1().min_w(theme::SPACE_0).px(theme::SPACE_2).py(theme::SPACE_1P5);
                match cells.get(i) {
                    Some(c) => cell.child(inline(ids, c, if is_head { theme::WEIGHT_MEDIUM } else { theme::WEIGHT_NORMAL })),
                    None => cell,
                }
            }))
    };
    div()
        .flex()
        .flex_col()
        .rounded(theme::RADIUS_MD)
        .border_1()
        .border_color(theme::border_subtle())
        .overflow_hidden()
        .text_size(theme::text_xs())
        .child(row(head, true))
        .children(rows.iter().map(|r| row(r, false)))
        .into_any_element()
}

/// Styled text; links are clickable and open in the browser.
fn inline(ids: &Ids, text: &Inline, weight: FontWeight) -> AnyElement {
    let runs = text
        .runs
        .iter()
        .map(|(r, s)| {
            let mut f = font(if s.code { theme::mono_font() } else { theme::ui_font() });
            f.weight = if s.bold { theme::WEIGHT_SEMIBOLD } else { weight };
            if s.italic {
                f.style = FontStyle::Italic;
            }
            let color = if s.link.is_some() {
                theme::accent()
            } else if s.muted {
                theme::text_subtle()
            } else {
                theme::text()
            };
            TextRun {
                len: r.len(),
                font: f,
                color,
                background_color: s.code.then_some(theme::surface_active()),
                underline: None::<UnderlineStyle>,
                strikethrough: s.strike.then(|| StrikethroughStyle { thickness: gpui::px(1.), color: Some(color) }),
            }
        })
        .collect();
    let styled = StyledText::new(text.text.clone()).with_runs(runs);
    let links = text.links();
    if links.is_empty() {
        return div().min_w(theme::SPACE_0).child(styled).into_any_element();
    }
    let (ranges, urls): (Vec<_>, Vec<_>) = links.into_iter().unzip();
    div()
        .min_w(theme::SPACE_0)
        .cursor_pointer()
        .child(InteractiveText::new(ids.next(), styled).on_click(ranges, move |ix, _, cx| cx.open_url(&urls[ix])))
        .into_any_element()
}
