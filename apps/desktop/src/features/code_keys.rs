//! Code tab line cursor: arrows move a one-line selection, Shift extends, c/Enter comment, Cmd+C copies.
//!
//! Outside the Code tab the arrows keep moving the PR list selection. Keyboard moves never animate.

use gpui::{actions, prelude::*, App, Context, KeyBinding};

use crate::app::GitViewer;
use crate::state::DetailTab;

actions!(
    git_viewer,
    [LineDown, LineUp, ExtendDown, ExtendUp, CopyLines]
);

pub fn bind_keys(cx: &mut App) {
    let list = Some("GitViewer && !TextInput");
    cx.bind_keys([
        KeyBinding::new("down", LineDown, list),
        KeyBinding::new("up", LineUp, list),
        KeyBinding::new("shift-down", ExtendDown, list),
        KeyBinding::new("shift-up", ExtendUp, list),
        KeyBinding::new("cmd-c", CopyLines, list),
    ]);
}

impl GitViewer {
    /// The Code tab (diff or whole-file viewer) is showing, so arrows drive the line cursor.
    pub(crate) fn code_lines_active(&self) -> bool {
        self.reviewing
            && self.detail.as_ref().is_some_and(|d| {
                d.tab == DetailTab::Code
                    && (d.code.viewer.is_some()
                        || d.code.files.as_ref().is_some_and(|f| !f.is_empty()))
            })
    }

    fn arrow(&mut self, delta: isize, extend: bool, cx: &mut Context<Self>) {
        if self.code_lines_active() {
            self.step_line(delta, extend, cx)
        } else if !extend {
            self.move_cursor(delta, cx)
        }
    }

    pub(crate) fn code_key_handlers<E: InteractiveElement>(
        &self,
        el: E,
        cx: &mut Context<Self>,
    ) -> E {
        el.on_action(cx.listener(|this, _: &LineDown, _, cx| this.arrow(1, false, cx)))
            .on_action(cx.listener(|this, _: &LineUp, _, cx| this.arrow(-1, false, cx)))
            .on_action(cx.listener(|this, _: &ExtendDown, _, cx| this.arrow(1, true, cx)))
            .on_action(cx.listener(|this, _: &ExtendUp, _, cx| this.arrow(-1, true, cx)))
            .on_action(cx.listener(|this, _: &CopyLines, _, cx| {
                if this.code_lines_active() {
                    this.copy_selection(cx)
                }
            }))
    }
}
