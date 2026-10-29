//! Floating toolbar at the end of a line selection: Comment, Copy, Copy link, Open in browser.

use gpui::{deferred, div, prelude::*, AnyElement, ClickEvent, Context};

use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::ui::atoms::{Button, ButtonVariant, IconName};
use crate::ui::molecules::FloatingToolbar;
use crate::ui::theme;

impl GitViewer {
    /// The toolbar hung below the last selected line, painted above later rows.
    pub(super) fn selection_bar(&self, comment: bool, animate: bool, cx: &mut Context<Self>) -> AnyElement {
        let btn = |id: &'static str, label: &'static str, icon: IconName, tip: &'static str| Button::new(id, label).icon(icon).variant(ButtonVariant::Ghost).tooltip(tip);
        let bar = FloatingToolbar::new("line-selection-bar")
            .animate(animate)
            .when(comment, |t| {
                t.child(btn("sel-comment", "Comment", IconName::Message, "Comment on lines  c").on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    this.comment_on_selection(window, cx)
                })))
            })
            .child(btn("sel-copy", "Copy", IconName::Copy, "Copy lines  \u{2318}C").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.copy_selection(cx))))
            .child(btn("sel-link", "", IconName::Link, "Copy link").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.copy_selection_link(cx))))
            .child(btn("sel-open", "", IconName::ExternalLink, "Open in browser").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                if let Some(url) = this.selection_link() {
                    cx.open_url(&url)
                }
            })));
        deferred(div().absolute().top(layout::DIFF_LINE_H).right(theme::SPACE_3).child(bar)).with_priority(1).into_any_element()
    }
}
