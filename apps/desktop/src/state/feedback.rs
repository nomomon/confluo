//! Short-lived feedback: "Copied" states and toasts that reset on a timer.

use gpui::{ClipboardItem, Context};

use super::timing;
use crate::app::GitViewer;

/// What was just copied, for inline "Copied" feedback.
#[derive(Clone, Copy, PartialEq)]
pub enum Copied {
    Link,
    Checkout,
    /// A file path in the Code tab.
    Path,
    /// The whole diff as a patch.
    Diff,
    /// Selected code lines.
    Lines,
}

impl GitViewer {
    /// Mark `what` as copied (the caller already wrote the clipboard); resets after `COPIED_RESET`.
    pub(crate) fn mark_copied(&mut self, what: Copied, cx: &mut Context<Self>) {
        self.copied = Some(what);
        self.copied_gen += 1;
        let gen = self.copied_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(timing::COPIED_RESET).await;
            this.update(cx, |v, cx| {
                if v.copied_gen == gen {
                    v.copied = None;
                    cx.notify()
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn copy(&mut self, what: Copied, text: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.mark_copied(what, cx);
        match what {
            Copied::Link => self.show_toast("Link copied", cx),
            Copied::Path => self.show_toast("Path copied", cx),
            Copied::Diff => self.show_toast("Diff copied", cx),
            Copied::Lines => self.show_toast("Copied", cx),
            Copied::Checkout => {}
        }
    }

    pub(crate) fn show_toast(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.toast = Some(text.into());
        self.toast_gen += 1;
        let gen = self.toast_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(timing::TOAST).await;
            this.update(cx, |v, cx| {
                if v.toast_gen == gen {
                    v.toast = None;
                    cx.notify()
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
