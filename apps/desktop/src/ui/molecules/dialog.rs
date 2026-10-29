//! Modal dialog: dimmed full-window scrim with a titled panel and close button.

use gpui::{div, prelude::*, px, AnyElement, App, ClickEvent, FontWeight, SharedString, Window};

use crate::ui::atoms::{Button, ButtonVariant, Size};
use crate::ui::{theme, ClickHandler};

#[derive(IntoElement)]
pub struct Dialog {
    title: SharedString,
    content: Option<AnyElement>,
    on_close: Option<ClickHandler>,
}

impl Dialog {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self { title: title.into(), content: None, on_close: None }
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn on_close(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Box::new(f));
        self
    }
}

impl RenderOnce for Dialog {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let mut close = Button::new("close-overlay", "").icon("\u{2715}").variant(ButtonVariant::Ghost).size(Size::Sm);
        if let Some(f) = self.on_close {
            close = close.on_click(f);
        }
        let header = div()
            .flex()
            .items_center()
            .child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(self.title))
            .child(div().flex_1())
            .child(close);
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme::scrim())
            .occlude()
            .child(
                div()
                    .w(px(520.))
                    .max_h(px(620.))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .rounded_xl()
                    .border_1()
                    .border_color(theme::border())
                    .bg(theme::card(0.97))
                    .shadow_lg()
                    .child(header)
                    .children(self.content),
            )
    }
}
