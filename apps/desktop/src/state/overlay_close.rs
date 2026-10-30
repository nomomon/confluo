//! Closing modal dialogs from the scrim, Esc or the close button: dialogs holding typed input ask
//! "Discard changes?" first.

use gpui::{App, Context, Window};

use crate::app::{GitViewer, Overlay};

/// What a close request does, given the open dialog and whether it holds typed input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CloseStep {
    /// Nothing open.
    None,
    /// Ask "Discard changes?" first.
    Ask,
    /// Close now.
    Close,
}

pub fn close_step(overlay: Overlay, dirty: bool, asking: bool) -> CloseStep {
    match overlay {
        Overlay::None => CloseStep::None,
        _ if dirty && !asking => CloseStep::Ask,
        _ => CloseStep::Close,
    }
}

impl GitViewer {
    /// The open dialog holds typed input that closing it would lose.
    pub(crate) fn overlay_dirty(&self, cx: &App) -> bool {
        let typed = |i: &gpui::Entity<crate::ui::molecules::text_input::TextInput>| {
            !i.read(cx).text().trim().is_empty()
        };
        match self.overlay {
            Overlay::ReviewSummary(_) => typed(&self.review_input),
            Overlay::AddProject => typed(&self.add_value),
            Overlay::Rename(id) => self
                .projects
                .iter()
                .find(|p| p.id == id)
                .is_some_and(|p| p.name != self.rename_input.read(cx).text().trim()),
            _ => false,
        }
    }

    /// Scrim press, Esc or the close button: close, or ask first when typed input would be lost.
    pub(crate) fn request_close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match close_step(self.overlay, self.overlay_dirty(cx), self.discard_prompt) {
            CloseStep::None => {}
            CloseStep::Ask => self.discard_prompt = true,
            CloseStep::Close => self.close_overlay_now(window, cx),
        }
        cx.notify();
    }

    /// Close the open dialog without asking (a confirm opened from Settings returns there).
    pub(crate) fn close_overlay_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.discard_prompt = false;
        if self.overlay == Overlay::Settings {
            self.overlay = Overlay::None;
        } else {
            self.close_confirm();
            if self.overlay != Overlay::Settings {
                self.overlay = Overlay::None;
            }
        }
        window.focus(&self.focus);
        cx.notify();
    }

    /// "Keep editing" in the discard prompt (also its scrim and Esc).
    pub(crate) fn keep_editing(&mut self, cx: &mut Context<Self>) {
        self.discard_prompt = false;
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_dialogs_ask_before_closing() {
        let review = Overlay::ReviewSummary(gv_engine::ReviewAction::Comment);
        assert_eq!(close_step(Overlay::None, false, false), CloseStep::None);
        assert_eq!(
            close_step(Overlay::Settings, false, false),
            CloseStep::Close
        );
        assert_eq!(close_step(review, false, false), CloseStep::Close);
        assert_eq!(close_step(review, true, false), CloseStep::Ask);
        // "Discard" in the prompt closes even though the input is still dirty.
        assert_eq!(close_step(review, true, true), CloseStep::Close);
    }
}
