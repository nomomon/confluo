//! `Block`s to GPUI elements with a vertical rhythm, styled only from `ui::theme`.

use gpui::{div, prelude::*, px, relative, AnyElement, Hsla, Pixels, SharedString};
use std::cell::Cell;

use super::blocks::{Alert, Block, Item};
use super::inline::Inline;
use super::table::table;
use super::text::MdText;
use super::widgets::{CodeBlock, Details};
use super::Images;
use crate::ui::atoms::{Checkbox, Icon, IconName, Separator};
use crate::ui::theme;

/// Unique element ids within one document, and the loaded images.
pub struct Ctx {
    prefix: SharedString,
    next: Cell<usize>,
    pub images: Images,
}

impl Ctx {
    pub fn id(&self) -> SharedString {
        let n = self.next.get();
        self.next.set(n + 1);
        format!("{}-md-{n}", self.prefix).into()
    }
}

pub fn render(prefix: &SharedString, blocks: &[Block], images: Images) -> Vec<AnyElement> {
    let ctx = Ctx { prefix: prefix.clone(), next: Cell::new(0), images };
    stack(&ctx, blocks, 0, false)
}

/// Space above a block that follows another one.
fn gap_before(b: &Block, tight: bool) -> Pixels {
    match b {
        Block::Heading(1 | 2, _) => theme::SPACE_6,
        Block::Heading(..) => theme::SPACE_5,
        _ if tight => theme::SPACE_1P5,
        _ => theme::SPACE_3,
    }
}

fn stack(ctx: &Ctx, blocks: &[Block], depth: usize, tight: bool) -> Vec<AnyElement> {
    blocks.iter().enumerate().map(|(i, b)| div().when(i > 0, |d| d.mt(gap_before(b, tight))).child(block(ctx, b, depth)).into_any_element()).collect()
}

fn text(ctx: &Ctx, t: &Inline) -> MdText {
    MdText::new(ctx.id(), t, theme::WEIGHT_NORMAL, ctx.images.clone())
}

fn alert_style(a: Alert) -> (Hsla, IconName) {
    match a {
        Alert::Note => (theme::accent(), IconName::InfoCircle),
        Alert::Tip => (theme::success(), IconName::Bulb),
        Alert::Important => (theme::merged(), IconName::AlertCircle),
        Alert::Warning => (theme::warning(), IconName::AlertTriangle),
        Alert::Caution => (theme::danger(), IconName::AlertCircle),
    }
}

fn block(ctx: &Ctx, b: &Block, depth: usize) -> AnyElement {
    match b {
        Block::Heading(level, t) => {
            let size = match level {
                1 => theme::text_xl(),
                2 => theme::text_lg(),
                3 => theme::text_base(),
                _ => theme::text_sm(),
            };
            let el = MdText::new(ctx.id(), t, theme::WEIGHT_SEMIBOLD, ctx.images.clone()).color(if *level >= 5 { theme::text_muted() } else { theme::text() });
            div().text_size(size).line_height(relative(1.3)).when(*level <= 2, |d| d.pb(theme::SPACE_1P5).border_b_1().border_color(theme::border_subtle())).child(el).into_any_element()
        }
        Block::Paragraph(t) => div().text_size(theme::text_sm()).line_height(relative(1.55)).child(text(ctx, t)).into_any_element(),
        Block::Code { lang, code } => CodeBlock::new(ctx.id(), lang, code).into_any_element(),
        Block::List { start, items } => div().flex().flex_col().gap(theme::SPACE_1).children(items.iter().enumerate().map(|(i, item)| list_item(ctx, *start, i, item, depth))).into_any_element(),
        Block::Quote { alert: None, blocks } => div().pl(theme::SPACE_3).border_l_2().border_color(theme::border()).text_color(theme::text_muted()).flex().flex_col().children(stack(ctx, blocks, depth, false)).into_any_element(),
        Block::Quote { alert: Some(a), blocks } => {
            let (color, icon) = alert_style(*a);
            let title = div().flex().items_center().gap(theme::SPACE_1P5).text_size(theme::text_sm()).font_weight(theme::WEIGHT_SEMIBOLD).text_color(color).child(Icon::new(icon).color(color)).child(a.label());
            div()
                .flex()
                .flex_col()
                .gap(theme::SPACE_1P5)
                .px(theme::SPACE_3)
                .py(theme::SPACE_2P5)
                .border_l_2()
                .border_color(color)
                .rounded_r(theme::RADIUS_MD)
                .bg(theme::tint_bg(color))
                .child(title)
                .children(stack(ctx, blocks, depth, true))
                .into_any_element()
        }
        Block::Table { aligns, head, rows } => table(ctx, aligns, head, rows),
        Block::Details { summary, blocks } => Details::new(ctx.id(), summary, stack(ctx, blocks, depth, false)).into_any_element(),
        Block::Footnote { label, blocks } => div()
            .flex()
            .gap(theme::SPACE_1P5)
            .text_size(theme::text_xs())
            .text_color(theme::text_muted())
            .child(format!("{label}."))
            .child(div().flex_1().flex().flex_col().children(stack(ctx, blocks, depth, true)))
            .into_any_element(),
        Block::Rule => div().my(theme::SPACE_1).child(Separator::new()).into_any_element(),
    }
}

fn bullet(depth: usize) -> &'static str {
    ["\u{2022}", "\u{25E6}", "\u{25AA}"][depth % 3]
}

fn list_item(ctx: &Ctx, start: Option<u64>, i: usize, item: &Item, depth: usize) -> AnyElement {
    let marker = match (item.task, start) {
        (Some(done), _) => div().pt(px(2.)).child(Checkbox::new(ctx.id(), done)).into_any_element(),
        (None, Some(n)) => div().text_color(theme::text_muted()).child(format!("{}.", n + i as u64)).into_any_element(),
        (None, None) => div().text_color(theme::text_subtle()).child(bullet(depth)).into_any_element(),
    };
    div()
        .flex()
        .gap(theme::SPACE_2)
        .text_size(theme::text_sm())
        .line_height(relative(1.55))
        .child(div().flex().justify_end().w(theme::SPACE_5).flex_shrink_0().child(marker))
        .child(div().flex().flex_col().flex_1().min_w(theme::SPACE_0).children(stack(ctx, &item.blocks, depth + 1, true)))
        .into_any_element()
}
