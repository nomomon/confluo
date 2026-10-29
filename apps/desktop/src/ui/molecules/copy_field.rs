//! Read-only monospace value with a copy-to-clipboard button.

use gpui::{div, prelude::*, px, App, ClickEvent, ClipboardItem, SharedString, Window};

use crate::ui::atoms::Button;
use crate::ui::{theme, ClickHandler};

#[derive(IntoElement)]
pub struct CopyField {
    id: SharedString,
    value: SharedString,
    copied: bool,
    on_copy: Option<ClickHandler>,
}

impl CopyField {
    pub fn new(id: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self { id: id.into(), value: value.into(), copied: false, on_copy: None }
    }

    /// Show the "Copied" label instead of "Copy".
    pub fn copied(mut self, copied: bool) -> Self {
        self.copied = copied;
        self
    }

    /// Called after the value was written to the clipboard.
    pub fn on_copy(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_copy = Some(Box::new(f));
        self
    }
}

impl RenderOnce for CopyField {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let value = self.value.clone();
        let on_copy = self.on_copy;
        div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .px(theme::SPACE_2)
                    .py(theme::SPACE_1)
                    .rounded(theme::RADIUS_MD)
                    .bg(theme::SURFACE)
                    .text_size(theme::TEXT_XS)
                    .font_family(theme::MONO_FONT)
                    .truncate()
                    .child(self.value),
            )
            .child(Button::new(self.id, if self.copied { "Copied" } else { "Copy" }).on_click(move |ev, window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(value.to_string()));
                if let Some(f) = &on_copy {
                    f(ev, window, cx)
                }
            }))
    }
}
