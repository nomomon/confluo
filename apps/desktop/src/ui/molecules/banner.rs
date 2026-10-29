//! One-line dismissible banner for non-fatal errors, with an optional action.
//!
//! `Banner::warning(id, text).action("Retry", on_retry).on_dismiss(on_dismiss)`

use gpui::{div, prelude::*, App, ClickEvent, SharedString, Window};

use crate::ui::atoms::{Button, ButtonVariant};
use crate::ui::{motion, theme, ClickHandler};

#[derive(IntoElement)]
pub struct Banner {
    id: SharedString,
    text: SharedString,
    action: Option<(SharedString, ClickHandler)>,
    on_dismiss: Option<ClickHandler>,
}

impl Banner {
    pub fn warning(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self { id: id.into(), text: text.into(), action: None, on_dismiss: None }
    }

    /// Inline link-style action such as "Retry".
    pub fn action(mut self, label: impl Into<SharedString>, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.action = Some((label.into(), Box::new(f)));
        self
    }

    pub fn on_dismiss(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(f));
        self
    }
}

impl RenderOnce for Banner {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let action = self.action.map(|(label, f)| Button::new(format!("{}-action", self.id), label).variant(ButtonVariant::Link).on_click(f));
        let dismiss = self.on_dismiss.map(|f| Button::new(format!("{}-dismiss", self.id), "").icon("\u{2715}").variant(ButtonVariant::Ghost).on_click(f));
        let el = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .text_size(theme::TEXT_XS)
            .text_color(theme::WARNING)
            .px(theme::SPACE_2)
            .py(theme::SPACE_1)
            .rounded(theme::RADIUS_MD)
            .bg(theme::tint_bg(theme::WARNING))
            .child(div().flex_1().min_w(theme::SPACE_0).truncate().child(self.text))
            .children(action)
            .children(dismiss);
        motion::fade_in(el, "banner")
    }
}
