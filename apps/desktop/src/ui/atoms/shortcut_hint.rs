//! Hold-Cmd shortcut hint: an inverted pill (light on dark, dark on light) floating over an element.
//!
//! `ShortcutHint::new("hint-refresh", "⌘ R").anchor(HintAnchor::Corner)`; place it with
//! `ShortcutHint::wrap(el, hint)` so the overlay is absolute and never shifts layout.

use gpui::{deferred, div, prelude::*, App, Div, ElementId, SharedString, Window};

use crate::ui::{motion, theme};

/// Where the pill sits relative to the wrapped element.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum HintAnchor {
    /// Overhanging the top-right corner (buttons).
    #[default]
    Corner,
    /// Inside the right edge, vertically centered (list items, inputs).
    Inline,
    /// Centered under the element (compact pairs).
    Below,
}

#[derive(IntoElement)]
pub struct ShortcutHint {
    id: ElementId,
    keys: Vec<SharedString>,
    anchor: HintAnchor,
}

impl ShortcutHint {
    /// `keys` uses the keymap display form (`⌘ ⇧ A`); spaces are dropped inside the pill.
    pub fn new(id: impl Into<ElementId>, keys: impl Into<SharedString>) -> Self {
        Self { id: id.into(), keys: vec![keys.into()], anchor: HintAnchor::default() }
    }

    /// Several shortcuts side by side (e.g. approve and request changes).
    pub fn pair(id: impl Into<ElementId>, keys: Vec<String>) -> Self {
        Self { id: id.into(), keys: keys.into_iter().map(Into::into).collect(), anchor: HintAnchor::default() }
    }

    pub fn anchor(mut self, anchor: HintAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// `el` in a relative box with the hint (if any) as an absolute overlay.
    pub fn wrap(el: impl IntoElement, hint: Option<ShortcutHint>) -> Div {
        div().relative().child(el).children(hint)
    }
}

fn pill(keys: &str) -> Div {
    div()
        .flex_none()
        .px(theme::SPACE_1P5)
        .py(theme::SPACE_0P5)
        .rounded(theme::RADIUS_SM)
        .bg(theme::text())
        .text_color(theme::bg())
        .shadow_md()
        .text_size(theme::text_hint())
        .font_family(theme::ui_font())
        .font_weight(theme::WEIGHT_MEDIUM)
        .whitespace_nowrap()
        .child(keys.split_whitespace().collect::<String>())
}

impl RenderOnce for ShortcutHint {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let row = div().flex().gap(theme::SPACE_1).children(self.keys.iter().map(|k| pill(k)));
        let placed = match self.anchor {
            HintAnchor::Corner => div().absolute().top(-theme::SPACE_2).right(-theme::SPACE_1P5).child(row),
            HintAnchor::Inline => div().absolute().top_0().bottom_0().right(theme::SPACE_2).flex().items_center().child(row),
            HintAnchor::Below => div().absolute().top_full().left_0().right_0().mt(theme::SPACE_1).flex().justify_center().child(row),
        };
        // A 60ms linear opacity (no easing or scale) so hints feel instant; final frame under reduced motion.
        deferred(motion::animate(placed, self.id, motion::BLINK, |el, t| el.opacity(t))).with_priority(3)
    }
}
