//! Side panel like shadcn `Sheet`: slides in from the right over a scrim (motion `PANEL`).
//!
//! `Sheet::new("Filters").description("..").on_close(..).child(..)`

use gpui::{div, prelude::*, px, AnyElement, App, ClickEvent, MouseButton, MouseDownEvent, Pixels, SharedString, Window};
use std::rc::Rc;

use crate::ui::atoms::{Button, ButtonVariant, IconName};
use crate::ui::{motion, theme};

const SHEET_W: Pixels = px(400.);

type CloseHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Sheet {
    title: SharedString,
    description: Option<SharedString>,
    width: Pixels,
    on_close: Option<CloseHandler>,
    children: Vec<AnyElement>,
}

impl Sheet {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self { title: title.into(), description: None, width: SHEET_W, on_close: None, children: vec![] }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    #[allow(dead_code)] // kit API
    pub fn width(mut self, w: Pixels) -> Self {
        self.width = w;
        self
    }

    /// Called from the close button and on a press on the scrim.
    pub fn on_close(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }
}

impl ParentElement for Sheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Sheet {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (c1, c2) = (self.on_close.clone(), self.on_close);
        let close = Button::new("sheet-close", "").icon(IconName::X).variant(ButtonVariant::Ghost).on_click(move |_: &ClickEvent, w, cx| {
            if let Some(f) = &c1 {
                f(w, cx)
            }
        });
        let header = div()
            .flex()
            .items_start()
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(theme::SPACE_1)
                    .child(div().text_size(theme::text_base()).font_weight(theme::WEIGHT_SEMIBOLD).child(self.title))
                    .children(self.description.map(|d| div().text_size(theme::text_sm()).text_color(theme::text_muted()).child(d))),
            )
            .child(close);
        let panel = div()
            .id("sheet-panel")
            .absolute()
            .top_0()
            .bottom_0()
            .right_0()
            .w(self.width)
            .flex()
            .flex_col()
            .gap(theme::SPACE_4)
            .p(theme::SPACE_5)
            .border_l_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_lg()
            .occlude()
            .overflow_y_scroll()
            .child(header)
            .children(self.children);
        let scrim = div().absolute().top_0().left_0().size_full().bg(theme::scrim()).occlude().on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, w, cx| {
            if let Some(f) = &c2 {
                f(w, cx)
            }
        });
        div().absolute().top_0().left_0().size_full().child(motion::fade_in(scrim, "sheet-scrim")).child(motion::slide_in_from_edge(panel, "sheet-slide", self.width))
    }
}
