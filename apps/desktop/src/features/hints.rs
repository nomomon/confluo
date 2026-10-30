//! Hold-Cmd shortcut hints: modifier tracking on the root view and the `hint` helpers features use
//! to put a `ShortcutHint` on a control. Keys and availability come from `keymap` only.

use gpui::{prelude::*, Action, Context, Div, ModifiersChangedEvent, Window};

use crate::app::GitViewer;
use crate::features::keymap::{self, Entry};
use crate::state::timing;
use crate::ui::atoms::{HintAnchor, ShortcutHint};

/// Display keys of an entry when it is a Cmd shortcut available in `ctx`.
fn cmd_keys(e: &Entry, ctx: &crate::state::cmd_ctx::CmdCtx) -> Option<String> {
    let first = e.keys.first()?;
    let is_cmd = |k: &str| k.split('-').any(|m| m == "cmd");
    // Every Cmd binding of the entry (Back shows both Cmd+[ and Cmd+Left).
    (is_cmd(first) && (e.available)(ctx)).then(|| e.keys.iter().filter(|k| is_cmd(k)).map(|k| keymap::display(k)).collect::<Vec<_>>().join("  "))
}

fn short_name<A: Action>() -> &'static str {
    let name = A::name_for_type();
    name.rsplit("::").next().unwrap_or(name)
}

impl GitViewer {
    pub(crate) fn hint_modifiers(&mut self, ev: &ModifiersChangedEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.settings.shortcut_hints {
            return;
        }
        let m = ev.modifiers;
        let was = self.hints.visible();
        if let Some(gen) = self.hints.modifiers(m.platform, m.shift || m.alt || m.control || m.function) {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(timing::HINT_HOLD).await;
                this.update(cx, |v, cx| {
                    if v.hints.fire(gen) {
                        cx.notify()
                    }
                })
                .ok();
            })
            .detach();
        }
        if was != self.hints.visible() {
            cx.notify()
        }
    }

    /// Shown keys for action `A` while hints are visible and the action is available.
    pub(crate) fn hint_keys<A: Action>(&self) -> Option<String> {
        let ctx = self.hint_ctx.as_ref()?;
        cmd_keys(keymap::entry(short_name::<A>())?, ctx)
    }

    /// Hint for action `A` at `anchor`, or `None` when hidden or unavailable.
    pub(crate) fn hint<A: Action>(&self, anchor: HintAnchor) -> Option<ShortcutHint> {
        self.hint_keys::<A>().map(|k| ShortcutHint::new(gpui::SharedString::from(format!("hint-{}", short_name::<A>())), k).anchor(anchor))
    }

    /// Several actions in one compact row (only the available ones), or `None`.
    pub(crate) fn hint_group(&self, id: &'static str, keys: Vec<Option<String>>, anchor: HintAnchor) -> Option<ShortcutHint> {
        let keys: Vec<String> = keys.into_iter().flatten().collect();
        (!keys.is_empty()).then(|| ShortcutHint::pair(id, keys).anchor(anchor))
    }

    /// Hint for the sidebar project at index `i` (Cmd+3..9).
    pub(crate) fn project_hint(&self, i: usize) -> Option<ShortcutHint> {
        let ctx = self.hint_ctx.as_ref()?;
        let keys = cmd_keys(keymap::project_entry(i)?, ctx)?;
        Some(ShortcutHint::new(gpui::SharedString::from(format!("hint-project-{i}")), keys).anchor(HintAnchor::Inline))
    }

    /// `el` with the hint for `A` overlaid (absolute; no layout shift).
    pub(crate) fn hinted<A: Action>(&self, el: impl IntoElement, anchor: HintAnchor) -> Div {
        ShortcutHint::wrap(el, self.hint::<A>(anchor))
    }
}
