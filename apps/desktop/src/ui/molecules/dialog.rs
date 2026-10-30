//! Modal dialog like shadcn `Dialog`: dimmed scrim, a panel with header (title, description,
//! close), content and an optional right-aligned footer.
//!
//! `Dialog::new("Delete project").description("..").content(..).footer(buttons).on_close(..)`

use gpui::{div, prelude::*, px, AnyElement, App, ClickEvent, Pixels, SharedString, Window};

use crate::ui::atoms::{Button, ButtonVariant, IconName, Size};
use crate::ui::{motion, theme, ClickHandler};

const DIALOG_W: Pixels = px(520.);
const DIALOG_MAX_H: Pixels = px(620.);

#[derive(IntoElement)]
pub struct Dialog {
    title: Option<SharedString>,
    description: Option<SharedString>,
    content: Option<AnyElement>,
    footer: Option<AnyElement>,
    size: Option<(Pixels, Pixels)>,
    on_close: Option<ClickHandler>,
}

impl Dialog {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: Some(title.into()),
            description: None,
            content: None,
            footer: None,
            size: None,
            on_close: None,
        }
    }

    /// A dialog without the title row; the content draws its own chrome (Settings).
    pub fn bare() -> Self {
        Self {
            title: None,
            description: None,
            content: None,
            footer: None,
            size: None,
            on_close: None,
        }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Fixed panel size instead of the default 520 wide, content high.
    pub fn size(mut self, w: Pixels, h: Pixels) -> Self {
        self.size = Some((w, h));
        self
    }

    pub fn on_close(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Box::new(f));
        self
    }
}

impl RenderOnce for Dialog {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let bare = self.title.is_none();
        let header = self.title.map(|title| {
            let mut close = Button::new("close-overlay", "")
                .icon(IconName::X)
                .variant(ButtonVariant::Ghost)
                .size(Size::Sm);
            if let Some(f) = self.on_close {
                close = close.on_click(f);
            }
            let text = div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(theme::SPACE_1)
                .child(
                    div()
                        .text_size(theme::text_base())
                        .font_weight(theme::WEIGHT_SEMIBOLD)
                        .child(title),
                )
                .children(self.description.map(|d| {
                    div()
                        .text_size(theme::text_sm())
                        .text_color(theme::text_muted())
                        .child(d)
                }));
            div().flex().items_start().child(text).child(close)
        });
        let panel = div()
            .flex()
            .flex_col()
            .rounded(theme::RADIUS_XL)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_lg()
            .overflow_hidden()
            .map(|d| match self.size {
                Some((w, h)) => d.w(w).h(h),
                None => d.w(DIALOG_W).max_h(DIALOG_MAX_H),
            })
            .when(!bare, |d| d.gap(theme::SPACE_4).p(theme::SPACE_5))
            .children(header)
            .children(self.content.map(|c| {
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(theme::SPACE_0)
                    .child(c)
            }))
            .children(
                self.footer
                    .map(|f| div().flex().justify_end().gap(theme::SPACE_2).child(f)),
            );
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
