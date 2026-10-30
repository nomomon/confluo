//! `?` cheat sheet: every shortcut by context (Global, List, Review, Diff, Palette), generated from
//! the keymap, with the context on screen first.

use gpui::{div, prelude::*, px, AnyElement, Context, Window};

use crate::app::{GitViewer, Overlay};
use crate::features::keymap::{group_rows, Group};
use crate::ui::atoms::{Badge, Kbd};
use crate::ui::molecules::Dialog;
use crate::ui::theme;

const SHEET_W: gpui::Pixels = px(640.);
const SHEET_H: gpui::Pixels = px(580.);

impl GitViewer {
    /// `?`, Help > Keyboard Shortcuts, or the palette: open (or close) the cheat sheet.
    pub(crate) fn open_shortcuts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.layer = None;
        self.overlay = if self.overlay == Overlay::Shortcuts { Overlay::None } else { Overlay::Shortcuts };
        window.focus(&self.focus);
        cx.notify();
    }

    /// The context group for what is on screen.
    pub(crate) fn current_group(&self) -> Group {
        if self.reviewing && self.detail.is_some() {
            Group::Review
        } else {
            Group::List
        }
    }

    /// Groups with the current context first (Review mode also lifts Diff).
    pub(crate) fn ordered_groups(&self) -> Vec<Group> {
        let cur = self.current_group();
        let mut first = vec![cur];
        if cur == Group::Review {
            first.push(Group::Diff);
        }
        first.into_iter().chain(Group::ALL.into_iter().filter(|g| *g != cur && !(cur == Group::Review && *g == Group::Diff))).collect()
    }

    pub(crate) fn render_shortcuts_sheet(&self) -> Dialog {
        let cur = self.current_group();
        let sections = self.ordered_groups().into_iter().map(|g| {
            let heading = div()
                .flex()
                .items_center()
                .gap(theme::SPACE_2)
                .pt(theme::section_gap())
                .pb(theme::section_header_gap())
                .text_size(theme::text_sm())
                .font_weight(theme::WEIGHT_MEDIUM)
                .text_color(theme::text_muted())
                .child(g.title())
                .when(g == cur, |h| h.child(Badge::new("Current").tint(theme::accent())));
            let rows = group_rows(g).into_iter().map(|(label, keys)| {
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(theme::SPACE_4)
                    .py(theme::item_pad_y())
                    .border_b_1()
                    .border_color(theme::border_subtle())
                    .child(div().text_color(theme::text()).child(label))
                    .child(div().flex().gap(theme::SPACE_1P5).children(keys.into_iter().map(Kbd::new)))
                    .into_any_element()
            });
            div().flex().flex_col().child(heading).children(rows).into_any_element()
        });
        let body: Vec<AnyElement> = sections.collect();
        Dialog::new("Keyboard shortcuts")
            .description("Press ? anywhere outside a text field. The palette (\u{2318} K) runs every command.")
            .size(SHEET_W, SHEET_H)
            .content(div().id("shortcuts-scroll").size_full().overflow_y_scroll().pr(theme::SPACE_2).children(body))
    }
}
