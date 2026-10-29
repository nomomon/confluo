//! Modal dialog: dimmed full-window scrim with a titled panel and close button.

use gpui::{div, prelude::*, px, AnyElement, Pixels, App, ClickEvent, SharedString, Window};

use crate::ui::atoms::{Button, ButtonVariant, IconName, Size};
use crate::ui::{motion, theme, ClickHandler};

const DIALOG_W: Pixels = px(520.);
const DIALOG_MAX_H: Pixels = px(620.);

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
        let mut close = Button::new("close-overlay", "").icon(IconName::X).variant(ButtonVariant::Ghost).size(Size::Sm);
        if let Some(f) = self.on_close {
            close = close.on_click(f);
        }
        let header = div()
            .flex()
            .items_center()
            .child(div().text_size(theme::text_base()).font_weight(theme::WEIGHT_SEMIBOLD).child(self.title))
            .child(div().flex_1())
            .child(close);
        let panel = div()
            .w(DIALOG_W)
            .max_h(DIALOG_MAX_H)
            .flex()
            .flex_col()
            .gap(theme::SPACE_3)
            .p(theme::SPACE_4)
            .rounded(theme::RADIUS_XL)
            .border_1()
            .border_color(theme::border_subtle())
            .bg(theme::overlay())
            .shadow_lg()
            .child(header)
            .children(self.content.map(|c| div().flex().flex_col().min_h(theme::SPACE_0).child(c)));
        let scrim = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme::scrim())
            .occlude()
            .child(motion::pop_in(panel, "dialog-panel"));
        motion::fade_in(scrim, "dialog-scrim")
    }
}
