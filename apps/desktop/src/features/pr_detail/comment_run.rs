//! Running a comment action (from the toolbar, the More menu or the right-click menu) on one comment.

use gpui::{Context, Focusable, Window};

use super::comment::Place;
use super::comment_actions::{quote, CommentAction, CommentFacts};
use crate::app::GitViewer;
use crate::state::Copied;

/// Everything an action needs about one comment.
#[derive(Clone)]
pub(super) struct CommentTarget {
    /// Surface plus id: which rendering owns the menu, picker and toolbar.
    pub key: crate::state::CommentKey,
    pub id: String,
    pub body: String,
    pub thread: Option<String>,
    pub place: Place,
    pub link: String,
    pub facts: CommentFacts,
    pub can_react: bool,
}

impl GitViewer {
    pub(super) fn run_comment_action(
        &mut self,
        a: CommentAction,
        t: &CommentTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.layer = None;
        match a {
            CommentAction::Reply | CommentAction::QuoteReply => {
                let Some(tid) = t.thread.clone() else { return };
                let text = if a == CommentAction::QuoteReply {
                    quote(&t.body)
                } else {
                    String::new()
                };
                match t.place {
                    Place::Activity => {
                        if let Some(d) = self.detail.as_mut() {
                            d.reply_to = Some(tid);
                            d.editing = None;
                        }
                        if !text.is_empty() {
                            self.composer.update(cx, |i, cx| i.set_text(text, cx));
                        }
                        window.focus(&self.composer.focus_handle(cx));
                    }
                    Place::Code => {
                        self.open_inline_field(Some(tid), None, window, cx);
                        if !text.is_empty() {
                            self.inline_input.update(cx, |i, cx| i.set_text(text, cx));
                        }
                    }
                }
            }
            CommentAction::Resolve | CommentAction::Unresolve => {
                if let Some(tid) = t.thread.clone() {
                    self.set_thread_resolved(tid, a == CommentAction::Resolve, cx)
                }
            }
            CommentAction::Edit => match t.place {
                Place::Activity => {
                    if let Some(d) = self.detail.as_mut() {
                        d.editing = Some(t.id.clone());
                        d.reply_to = None;
                    }
                    let body = t.body.clone();
                    self.composer.update(cx, |i, cx| i.set_text(body, cx));
                    window.focus(&self.composer.focus_handle(cx));
                }
                Place::Code => {
                    self.open_inline_field(None, Some((t.id.clone(), t.body.clone())), window, cx)
                }
            },
            CommentAction::Delete => self.confirm_delete_comment(t.id.clone(), cx),
            CommentAction::CopyText => {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(t.body.clone()));
                self.show_toast("Copied comment text", cx);
            }
            CommentAction::CopyLink => self.copy(Copied::Link, t.link.clone(), cx),
            CommentAction::OpenInBrowser => cx.open_url(&t.link),
            CommentAction::ShowInCode => {
                if let Some(tid) = t.thread.clone() {
                    self.show_thread_in_code(tid, cx)
                }
            }
        }
        cx.notify();
    }
}
