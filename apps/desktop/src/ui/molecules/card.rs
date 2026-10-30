//! Bordered translucent card like shadcn `Card`, with optional header, footer and flush rows.
//!
//! `Card::new(surfaces).title("Checkout").description("..").action(btn).child(..).footer(..)`;
//! `.rows()` lays children flush and splits them with hairline dividers (settings, key/value).

use gpui::{div, prelude::*, AnyElement, App, Hsla, SharedString, Window};

use crate::ui::theme::{self, Translucency};

#[derive(IntoElement)]
pub struct Card {
    fill: Hsla,
    rows: bool,
    title: Option<SharedString>,
    description: Option<SharedString>,
    action: Option<AnyElement>,
    footer: Option<AnyElement>,
    children: Vec<AnyElement>,
}

impl Card {
    /// Cards take their fill from the appearance-derived translucency.
    pub fn new(surfaces: Translucency) -> Self {
        Self {
            fill: surfaces.card(),
            rows: false,
            title: None,
            description: None,
            action: None,
            footer: None,
            children: vec![],
        }
    }

    /// Flush rows separated by dividers instead of padded content.
    pub fn rows(mut self) -> Self {
        self.rows = true;
        self
    }

    pub fn title(mut self, t: impl Into<SharedString>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn description(mut self, t: impl Into<SharedString>) -> Self {
        self.description = Some(t.into());
        self
    }

    /// Element at the right of the header.
    #[allow(dead_code)] // kit API
    pub fn action(mut self, el: impl IntoElement) -> Self {
        self.action = Some(el.into_any_element());
        self
    }

    #[allow(dead_code)] // kit API
    pub fn footer(mut self, el: impl IntoElement) -> Self {
        self.footer = Some(el.into_any_element());
        self
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Card {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let base = div()
            .w_full()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .rounded(theme::LIST_RADIUS)
            .border_1()
            .border_color(theme::card_border())
            .bg(self.fill);
        let header = (self.title.is_some() || self.action.is_some()).then(|| {
            let text = div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(theme::space_0p5())
                .children(self.title.map(|t| {
                    div()
                        .text_size(theme::text_sm())
                        .font_weight(theme::WEIGHT_MEDIUM)
                        .child(t)
                }))
                .children(self.description.map(|d| {
                    div()
                        .text_size(theme::text_xs())
                        .text_color(theme::text_muted())
                        .child(d)
                }));
            div()
                .flex()
                .items_center()
                .gap(theme::space_3())
                .child(text)
                .children(self.action)
        });
        let footer = self.footer.map(|f| {
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(theme::space_2())
                .child(f)
        });
        if self.rows {
            let header = header.map(|h| {
                h.px(theme::card_pad())
                    .pt(theme::space_3())
                    .pb(theme::space_2())
                    .into_any_element()
            });
            let footer = footer.map(|f| {
                f.px(theme::card_pad())
                    .py(theme::space_3())
                    .into_any_element()
            });
            let rows = self.children.into_iter().enumerate().map(|(i, c)| {
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .when(i > 0, |d| {
                        d.border_t_1().border_color(theme::border_subtle())
                    })
                    .child(c)
                    .into_any_element()
            });
            base.overflow_hidden()
                .children(header)
                .children(rows)
                .children(footer.map(|f| {
                    div()
                        .border_t_1()
                        .border_color(theme::border_subtle())
                        .child(f)
                }))
        } else {
            base.gap(theme::space_2p5())
                .p(theme::card_pad())
                .children(header)
                .children(self.children)
                .children(footer)
        }
    }
}
