//! One-line dismissible banner, with an optional action: `warning` for non-fatal errors (amber tint),
//! `info` for neutral state such as an active filter (hairline, muted text).
//!
//! `Banner::warning(id, text).action("Retry", on_retry).on_dismiss(on_dismiss)`
//! `Banner::info(id, text).icon(IconName::GitCommit).on_dismiss(on_dismiss)`

use gpui::{div, prelude::*, App, ClickEvent, SharedString, Window};

use crate::ui::atoms::{Button, ButtonVariant, Icon, IconName};
use crate::ui::{motion, theme, ClickHandler};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tone {
    Warning,
    Info,
}

#[derive(IntoElement)]
pub struct Banner {
    id: SharedString,
    text: SharedString,
    tone: Tone,
    icon: Option<IconName>,
    action: Option<(SharedString, ClickHandler)>,
    on_dismiss: Option<ClickHandler>,
}

impl Banner {
    pub fn warning(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            tone: Tone::Warning,
            icon: None,
            action: None,
            on_dismiss: None,
        }
    }

    /// Neutral banner for state that is not a problem.
    pub fn info(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            tone: Tone::Info,
            ..Self::warning(id, text)
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Inline link-style action such as "Retry".
    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
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
        let action = self.action.map(|(label, f)| {
            Button::new(format!("{}-action", self.id), label)
                .variant(ButtonVariant::Link)
                .on_click(f)
        });
        let dismiss = self.on_dismiss.map(|f| {
            Button::new(format!("{}-dismiss", self.id), "")
                .icon(IconName::X)
                .variant(ButtonVariant::Ghost)
                .on_click(f)
        });
        let fg = match self.tone {
            Tone::Warning => theme::warning(),
            Tone::Info => theme::text_muted(),
        };
        let el = div()
            .flex()
            .items_center()
            .gap(theme::space_2())
            .text_size(theme::text_xs())
            .text_color(fg)
            .px(theme::space_2())
            .py(theme::space_1())
            .rounded(theme::RADIUS_MD)
            .map(|d| match self.tone {
                Tone::Warning => d.bg(theme::tint_bg(theme::warning())),
                Tone::Info => d
                    .bg(theme::container_bg())
                    .border_1()
                    .border_color(theme::border()),
            })
            .children(
                self.icon
                    .map(|i| Icon::new(i).size(theme::icon_sm()).color(fg)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(theme::space_0())
                    .truncate()
                    .child(self.text),
            )
            .children(action)
            .children(dismiss);
        motion::fade_in(el, "banner")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tones() {
        assert_eq!(Banner::warning("a", "t").tone, Tone::Warning);
        let b = Banner::info("a", "t").icon(IconName::GitCommit);
        assert_eq!(
            (b.tone, b.icon, b.text.as_ref()),
            (Tone::Info, Some(IconName::GitCommit), "t")
        );
    }
}
