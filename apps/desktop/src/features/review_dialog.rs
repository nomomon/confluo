//! Review actions in the detail header: a Review menu button, my state badge, and the summary dialog.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, MouseDownEvent};
use gv_engine::{ReviewAction, ReviewInfo, ReviewState};

use super::keymap::{self, Approve, MergePr, RequestChanges, ToggleDraft};
use crate::app::{GitViewer, Menu, Overlay};
use crate::state::DetailState;
use crate::ui::atoms::{Badge, Button, ButtonVariant, IconName};
use crate::ui::molecules::{Dialog, DropdownMenu, MenuItem, MenuSeparator, Textarea};
use crate::ui::theme;

/// Badge text and color for my current review.
fn state_badge(s: ReviewState) -> Option<(&'static str, gpui::Hsla)> {
    match s {
        ReviewState::Approved => Some(("You approved", theme::success())),
        ReviewState::ChangesRequested => Some(("You requested changes", theme::danger())),
        ReviewState::Commented => Some(("You reviewed", theme::text_muted())),
        ReviewState::Pending => None,
    }
}

/// Review button tooltip: the review shortcuts.
fn review_tip() -> String {
    let k = |s: String| s.replace(' ', "");
    format!("Approve {}, request changes {}, merge {}, draft {}", k(keymap::kbd::<Approve>()), k(keymap::kbd::<RequestChanges>()), k(keymap::kbd::<MergePr>()), k(keymap::kbd::<ToggleDraft>()))
}

impl GitViewer {
    /// The Review button, or nothing when the provider offers no review actions here.
    pub(crate) fn render_review_button(&self, d: &DetailState, cx: &mut Context<Self>) -> Option<AnyElement> {
        let info = &d.detail.as_ref()?.review;
        if !info.caps.any() {
            return None;
        }
        let open = self.menu == Some(Menu::Review);
        let btn = Button::new("review-menu", if d.posting { "Submitting..." } else { "Review" }).variant(ButtonVariant::Primary).trailing_icon(IconName::ChevronDown).tooltip(review_tip()).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.menu = if this.menu == Some(Menu::Review) { None } else { Some(Menu::Review) };
            cx.notify()
        }));
        let hint = self.hint_group("hint-review", vec![self.hint_keys::<Approve>(), self.hint_keys::<RequestChanges>()], crate::ui::atoms::HintAnchor::Below).filter(|_| !open);
        Some(div().relative().child(btn).children(hint).when(open, |el| el.child(gpui::deferred(self.render_review_menu(info, cx)).with_priority(1))).into_any_element())
    }

    fn render_review_menu(&self, info: &ReviewInfo, cx: &mut Context<Self>) -> impl IntoElement {
        let c = &info.caps;
        let mut menu = DropdownMenu::new("review-dropdown").top(theme::SPACE_8).right(theme::SPACE_0).on_dismiss(cx.listener(|this, _: &MouseDownEvent, _, cx| {
            this.menu = None;
            cx.notify()
        }));
        if c.approve || c.approve_blocked.is_some() {
            let mut item = MenuItem::new("review-approve", "Approve").icon(IconName::CircleCheck).shortcut(keymap::kbd::<Approve>()).disabled(!c.approve);
            if let Some(why) = &c.approve_blocked {
                item = item.tooltip(why.clone());
            }
            menu = menu.child(item.on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.submit_review(ReviewAction::Approve, cx))));
        }
        if c.approve_with_suggestions {
            menu = menu.child(MenuItem::new("review-suggest", "Approve with suggestions").icon(IconName::CircleCheck).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.submit_review(ReviewAction::ApproveWithSuggestions, cx)
            })));
        }
        if c.wait_for_author {
            menu = menu.child(MenuItem::new("review-wait", "Wait for author").icon(IconName::Clock).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.submit_review(ReviewAction::WaitForAuthor, cx)
            })));
        }
        if c.request_changes {
            menu = menu.child(MenuItem::new("review-changes", c.request_changes_label().to_string()).icon(IconName::CircleX).shortcut(keymap::kbd::<RequestChanges>()).on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.open_review_summary(ReviewAction::RequestChanges, window, cx)
            })));
        }
        if c.comment_review {
            menu = menu.child(MenuItem::new("review-comment", "Comment").icon(IconName::MessageCircle).on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.open_review_summary(ReviewAction::Comment, window, cx)
            })));
        }
        if c.reset_vote {
            menu = menu.child(MenuSeparator).child(MenuItem::new("review-reset", "Reset vote").icon(IconName::ArrowBackUp).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.submit_review(ReviewAction::ResetVote, cx)
            })));
        }
        if c.unapprove {
            menu = menu.child(MenuSeparator).child(MenuItem::new("review-revoke", "Revoke approval").icon(IconName::ArrowBackUp).destructive().on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.submit_review(ReviewAction::Unapprove, cx)
            })));
        }
        menu
    }

    /// "You approved" for the meta line (approval counts are the single `approvals` indicator).
    pub(crate) fn render_review_state(&self, d: &DetailState) -> Option<AnyElement> {
        let info = d.detail.as_ref().map(|x| &x.review)?;
        info.my_state.and_then(state_badge).map(|(label, color)| Badge::new(label).tint(color).into_any_element())
    }

    /// Summary dialog for Comment and Request changes.
    pub(crate) fn render_review_dialog(&self, action: ReviewAction, cx: &mut Context<Self>) -> Dialog {
        let empty = self.review_input.read(cx).text().trim().is_empty();
        let caps = self.detail.as_ref().and_then(|d| d.detail.as_ref()).map(|x| x.review.caps.clone()).unwrap_or_default();
        // GitHub needs a body for these two; an empty submit would be rejected. Azure votes take an optional comment.
        let needs_body = matches!(action, ReviewAction::Comment | ReviewAction::RequestChanges) && caps.request_changes_label.is_none();
        let label = if action == ReviewAction::RequestChanges { caps.request_changes_label().to_string() } else { action.label().to_string() };
        let submit = Button::new("review-submit", label.clone()).variant(if action == ReviewAction::RequestChanges { ButtonVariant::Destructive } else { ButtonVariant::Primary });
        let submit = if needs_body && empty { submit.tooltip("Write a summary first") } else { submit.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.submit_review(action, cx))) };
        let footer = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .child(div().text_size(theme::text_xs()).text_color(theme::text_subtle()).child(format!("{} to submit", keymap::display("cmd-enter"))))
            .child(Button::new("review-cancel", "Cancel").variant(ButtonVariant::Ghost).on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.overlay = Overlay::None;
                window.focus(&this.focus);
                cx.notify()
            })))
            .child(div().when(needs_body && empty, |d| d.opacity(0.5)).child(submit));
        let title = match action {
            ReviewAction::RequestChanges => label,
            _ => "Submit review".to_string(),
        };
        let desc = if needs_body { "Your summary is posted with the review" } else { "An optional comment is posted with your vote" };
        Dialog::new(title).description(desc).content(Textarea::new(self.review_input.clone()).rows(4)).footer(footer)
    }
}
