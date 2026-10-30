//! `TextInput` with adornments like shadcn's input group: a leading icon and a clear button
//! shown while it has text. Create the entity with `TextInput::new(..).inset(INSET, INSET)`.
//!
//! `Input::new("search", search.clone(), has_text).leading(IconName::Search).on_clear(..)`

use gpui::{div, prelude::*, px, App, ClickEvent, Entity, Pixels, SharedString, Window};

use super::text_input::TextInput;
use crate::ui::atoms::{Button, ButtonVariant, Icon, IconName};
use crate::ui::{theme, ClickHandler};

/// Padding to give the inner `TextInput` on an adorned side.
pub const INSET: Pixels = px(34.);

#[derive(IntoElement)]
pub struct Input {
    id: SharedString,
    input: Entity<TextInput>,
    has_text: bool,
    leading: Option<IconName>,
    clear_tooltip: Option<SharedString>,
    on_clear: Option<ClickHandler>,
}

impl Input {
    pub fn new(id: impl Into<SharedString>, input: Entity<TextInput>, has_text: bool) -> Self {
        Self { id: id.into(), input, has_text, leading: None, clear_tooltip: None, on_clear: None }
    }

    pub fn leading(mut self, icon: IconName) -> Self {
        self.leading = Some(icon);
        self
    }

    pub fn clear_tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.clear_tooltip = Some(text.into());
        self
    }

    /// Show a clear button while there is text.
    pub fn on_clear(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_clear = Some(Box::new(f));
        self
    }
}

impl RenderOnce for Input {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let side = || div().absolute().top_0().bottom_0().flex().items_center();
        let clear = self.on_clear.filter(|_| self.has_text).map(|f| {
            let b = Button::new(format!("{}-clear", self.id), "").icon(IconName::X).variant(ButtonVariant::Ghost).on_click(f);
            side().right(theme::SPACE_1P5).child(match self.clear_tooltip {
                Some(t) => b.tooltip(t),
                None => b,
            })
        });
        div()
            .relative()
            .flex_1()
            .child(self.input)
            .children(self.leading.map(|n| side().left(theme::SPACE_3).child(Icon::new(n).color(theme::text_subtle()))))
            .children(clear)
    }
}
