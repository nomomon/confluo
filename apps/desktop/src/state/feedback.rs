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
    /// The source branch name.
    Branch,
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
            Copied::Branch => self.show_toast("Branch copied", cx),
            Copied::Checkout => {}
        }
    }

    pub(crate) fn show_toast(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.toast_error = None;
        self.toast_plain = false;
        self.start_toast(text.into(), timing::TOAST, cx);
    }

    /// Neutral toast without an icon: list ends, or why a shortcut is not possible.
    pub(crate) fn show_info_toast(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.toast_error = None;
        self.toast_plain = true;
        self.start_toast(text.into(), timing::TOAST, cx);
    }

    /// Red toast with the provider's reason; stays longer than a success toast.
    pub(crate) fn show_error_toast(
        &mut self,
        title: impl Into<String>,
        reason: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        let reason: String = reason.into();
        self.toast_error = Some(reason.chars().take(200).collect());
        self.toast_plain = false;
        self.start_toast(title.into(), timing::TOAST_ERROR, cx);
    }

    /// Quiet notice that the last run crashed, with "Open log".
    pub(crate) fn show_crash_toast(&mut self, log: std::path::PathBuf, cx: &mut Context<Self>) {
        self.toast_error = None;
        self.toast_plain = true;
        self.start_toast(
            "Confluo quit unexpectedly last time.".into(),
            timing::TOAST_ERROR,
            cx,
        );
        self.toast_file = Some(log);
    }

    fn start_toast(&mut self, text: String, after: std::time::Duration, cx: &mut Context<Self>) {
        self.toast = Some(text);
        self.toast_file = None;
        self.quiet.undo = None;
        self.toast_gen += 1;
        let gen = self.toast_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(after).await;
            this.update(cx, |v, cx| {
                if v.toast_gen == gen {
                    v.toast = None;
                    v.toast_file = None;
                    v.quiet.undo = None;
                    cx.notify()
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
