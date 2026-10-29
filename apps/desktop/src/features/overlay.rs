//! Modal overlay host: picks the dialog for the current `Overlay`.

use gpui::{prelude::*, AnyElement, ClickEvent, Context};

use crate::app::{GitViewer, Overlay};
use crate::ui::molecules::Dialog;

impl GitViewer {
    pub(crate) fn render_overlay(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (title, content): (&str, AnyElement) = match self.overlay {
            Overlay::None => return None,
            Overlay::Settings => ("Settings", self.render_settings(cx).into_any_element()),
            Overlay::AddProject => ("Add project", self.render_add_project(cx).into_any_element()),
            Overlay::Alias => ("Repo alias", self.render_alias(cx).into_any_element()),
            Overlay::Rename(_) => ("Rename project", self.render_rename(cx).into_any_element()),
        };
        Some(
            Dialog::new(title)
                .content(content)
                .on_close(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.overlay = Overlay::None;
                    cx.notify()
                }))
                .into_any_element(),
        )
    }
}
