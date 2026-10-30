//! Inline markdown text: styled runs, links that underline on hover and open on click, and
//! images (loaded ones drawn with a max width, others as alt text).

use gpui::{
    div, font, img, prelude::*, px, AnyElement, App, ClickEvent, ElementId, FontStyle, FontWeight, Hsla, InteractiveText, MouseMoveEvent, SharedString, StrikethroughStyle,
    StyledText, TextRun, UnderlineStyle, Window,
};
use std::ops::Range;

use super::inline::{Inline, Style};
use super::Images;
use crate::ui::atoms::{Icon, IconName};
use crate::ui::theme;

const IMAGE_MAX_H: gpui::Pixels = px(420.);

#[derive(IntoElement)]
pub struct MdText {
    id: SharedString,
    runs: Vec<(String, Style)>,
    weight: FontWeight,
    color: Hsla,
    images: Images,
}

impl MdText {
    pub fn new(id: SharedString, inline: &Inline, weight: FontWeight, images: Images) -> Self {
        let runs = inline.runs.iter().map(|(r, s)| (inline.text[r.clone()].to_string(), s.clone())).collect();
        Self { id, runs, weight, color: theme::text(), images }
    }

    /// Color of plain runs (links and code keep theirs).
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }
}

fn text_run(len: usize, s: &Style, weight: FontWeight, base: Hsla, hovered: bool) -> TextRun {
    let mut f = font(if s.code { theme::mono_font() } else { theme::ui_font() });
    f.weight = if s.bold { theme::WEIGHT_SEMIBOLD } else { weight };
    if s.italic {
        f.style = FontStyle::Italic;
    }
    let color = if s.link.is_some() {
        theme::accent()
    } else if s.image.is_some() {
        theme::text_subtle()
    } else {
        base
    };
    TextRun {
        len,
        font: f,
        color,
        background_color: s.code.then_some(theme::surface_active()),
        underline: hovered.then(|| UnderlineStyle { thickness: px(1.), color: Some(color), wavy: false }),
        strikethrough: s.strike.then(|| StrikethroughStyle { thickness: px(1.), color: None }),
    }
}

/// One wrapping text element for runs without images.
fn text_segment(id: ElementId, runs: &[(String, Style)], weight: FontWeight, base: Hsla, hovered: Option<usize>, on_hover: impl Fn(Option<usize>, &mut App) + 'static) -> AnyElement {
    let mut text = String::new();
    let mut links: Vec<(Range<usize>, String)> = vec![];
    let mut tr = vec![];
    for (t, s) in runs {
        let start = text.len();
        text.push_str(t);
        let link_ix = s.link.as_ref().map(|l| {
            links.push((start..text.len(), l.clone()));
            links.len() - 1
        });
        tr.push(text_run(t.len(), s, weight, base, link_ix.is_some() && link_ix == hovered));
    }
    let styled = StyledText::new(text).with_runs(tr);
    if links.is_empty() {
        return div().min_w(theme::SPACE_0).child(styled).into_any_element();
    }
    let (ranges, urls): (Vec<_>, Vec<_>) = links.into_iter().unzip();
    let hover_ranges = ranges.clone();
    let interactive = InteractiveText::new(id, styled)
        .on_click(ranges, move |ix, _, cx| cx.open_url(&urls[ix]))
        .on_hover(move |ix, _: MouseMoveEvent, _, cx| on_hover(ix.and_then(|i| hover_ranges.iter().position(|r| r.contains(&i))), cx));
    div().min_w(theme::SPACE_0).cursor_pointer().child(interactive).into_any_element()
}

impl RenderOnce for MdText {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let hover = window.use_keyed_state(ElementId::Name(format!("{}-hover", self.id).into()), cx, |_, _| None::<usize>);
        let hovered = *hover.read(cx);
        let set = move |h: Option<usize>, cx: &mut App| {
            hover.update(cx, |v, cx| {
                if *v != h {
                    *v = h;
                    cx.notify()
                }
            })
        };
        if !self.runs.iter().any(|(_, s)| s.image.is_some()) {
            return text_segment(ElementId::Name(self.id), &self.runs, self.weight, self.color, hovered, set);
        }
        // Mixed text and images: a wrapping row of text segments and pictures.
        let mut row = div().flex().flex_wrap().items_center().gap(theme::SPACE_1).min_w(theme::SPACE_0);
        let mut pending: Vec<(String, Style)> = vec![];
        let mut seg = 0;
        let flush = |row: gpui::Div, pending: &mut Vec<(String, Style)>, seg: &mut usize| {
            if pending.iter().all(|(t, _)| t.trim().is_empty()) {
                pending.clear();
                return row;
            }
            *seg += 1;
            let el = text_segment(ElementId::Name(format!("{}-s{seg}", self.id).into()), pending, self.weight, self.color, None, |_, _| {});
            pending.clear();
            row.child(el)
        };
        for (i, (t, s)) in self.runs.iter().enumerate() {
            let Some(src) = &s.image else {
                pending.push((t.clone(), s.clone()));
                continue;
            };
            row = flush(row, &mut pending, &mut seg);
            let pic: AnyElement = match self.images.get(src) {
                Some(image) => img(image.clone()).max_w_full().max_h(IMAGE_MAX_H).rounded(theme::RADIUS_SM).into_any_element(),
                None => div().flex().items_center().gap(theme::SPACE_1).text_color(theme::text_subtle()).child(Icon::new(IconName::Photo).size(theme::ICON_SM)).child(t.clone()).into_any_element(),
            };
            let link = s.link.clone();
            row = row.child(div().id(ElementId::Name(format!("{}-img{i}", self.id).into())).child(pic).when_some(link, |d, l| d.cursor_pointer().on_click(move |_: &ClickEvent, _, cx| cx.open_url(&l))));
        }
        flush(row, &mut pending, &mut seg).into_any_element()
    }
}
