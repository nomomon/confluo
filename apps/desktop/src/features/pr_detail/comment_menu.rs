//! Running comment actions and the menu built from the registry (`comment_actions`): reactions as a row of
//! emoji buttons on top, then Reply / Quote reply, Resolve, copy and links, Show in code, Edit / Delete last.
//! Right-click shows everything at the pointer; the hover toolbar's More shows only what the toolbar lacks.

use gpui::{
    div, prelude::*, AnyElement, ClickEvent, Context, Focusable, MouseDownEvent, SharedString,
    Window,
};
use gv_engine::ReactionKind;

use super::comment::Place;
use super::comment_actions::{quote, CommentAction, CommentFacts};
use crate::app::{GitViewer, Layer};
use crate::state::Copied;
use crate::ui::molecules::{ContextMenu, DropdownMenu, MenuItem, MenuSeparator};
use crate::ui::theme;

pub(super) const EMOJI_FONT: &str = "Apple Color Emoji";

/// Everything an action needs about one comment.
#[derive(Clone)]
pub(super) struct CommentTarget {
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

    /// Menu rows for `t`. Right-click (`full`): emoji row, then every available action. The toolbar's
    /// More: only actions the toolbar does not show (no emoji row, no Reply / Resolve).
    fn comment_menu_items(
        &self,
        t: &CommentTarget,
        full: bool,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut out: Vec<AnyElement> = vec![];
        if t.can_react && full {
            let kind = self
                .detail
                .as_ref()
                .map(|d| d.summary.id.repo.kind)
                .unwrap_or(gv_engine::ProviderKind::GitHub);
            let emoji = ReactionKind::for_provider(kind).iter().copied().map(|k| {
                let id = t.id.clone();
                div()
                    .id(SharedString::from(format!("cm-rx-{}-{k:?}", t.id)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(theme::space_7())
                    .rounded(theme::RADIUS_MD)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme::surface_hover()))
                    .font_family(EMOJI_FONT)
                    .text_size(theme::text_sm())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.layer = None;
                        this.toggle_reaction(Some(id.clone()), k, cx)
                    }))
                    .child(k.emoji())
            });
            out.push(
                div()
                    .flex()
                    .px(theme::space_1())
                    .children(emoji)
                    .into_any_element(),
            );
            out.push(MenuSeparator.into_any_element());
        }
        let groups = if full {
            CommentAction::menu(&t.facts)
        } else {
            CommentAction::more_menu(&t.facts)
        };
        for (gi, group) in groups.into_iter().enumerate() {
            if gi > 0 {
                out.push(MenuSeparator.into_any_element());
            }
            for a in group {
                let t2 = t.clone();
                let mut item =
                    MenuItem::new(SharedString::from(format!("cm-{a:?}-{}", t.id)), a.label())
                        .icon(a.icon());
                if a.destructive() {
                    item = item.destructive();
                }
                out.push(
                    item.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.run_comment_action(a, &t2, window, cx)
                    }))
                    .into_any_element(),
                );
            }
        }
        out
    }

    /// The open menu of comment `t`, if any: anchored under More, or at the right-click point.
    pub(super) fn comment_menu(
        &self,
        t: &CommentTarget,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let dismiss = cx.listener(|this, _: &MouseDownEvent, _, cx| this.dismiss_layer(cx));
        match &self.layer {
            Some(Layer::CommentMenu(id)) if *id == t.id => {
                let items = self.comment_menu_items(t, false, cx);
                Some(
                    DropdownMenu::new(SharedString::from(format!("comment-menu-{}", t.id)))
                        .on_dismiss(dismiss)
                        .children(items)
                        .into_any_element(),
                )
            }
            Some(Layer::CommentMenuAt(id, at)) if *id == t.id => {
                let items = self.comment_menu_items(t, true, cx);
                Some(
                    ContextMenu::new(SharedString::from(format!("comment-ctx-{}", t.id)), *at)
                        .on_dismiss(dismiss)
                        .children(items)
                        .into_any_element(),
                )
            }
            _ => None,
        }
    }
}
