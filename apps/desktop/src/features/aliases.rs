//! Repo alias dialog: pick a repo, edit its display alias.

use gpui::{div, prelude::*, ClickEvent, Context};

use crate::app::GitViewer;
use crate::ui::atoms::{Button, NavItem};
use crate::ui::molecules::Field;
use crate::ui::theme;
use crate::features::shared::layout;

impl GitViewer {
    pub(super) fn render_alias(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div().id("alias-list").flex().flex_col().max_h(layout::ALIAS_LIST_MAX_H).overflow_y_scroll().gap(theme::SPACE_0P5);
        let cands = self.alias_candidates();
        if cands.is_empty() {
            list = list.child(div().text_size(theme::text_sm()).text_color(theme::text_subtle()).child("No repos in view. Select a project or refresh first."));
        }
        for r in cands {
            let label = match self.engine.repo_override(&r).ok().and_then(|o| o.alias) {
                Some(a) => format!("{}  ({a})", r.path),
                None => r.path.clone(),
            };
            let active = self.alias_repo.as_ref() == Some(&r);
            list = list.child(
                NavItem::new(format!("alias-{r}"), label)
                    .active(active)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_alias_repo(Some(r.clone()), cx))),
            );
        }
        let editor = self.alias_repo.as_ref().map(|r| {
            Field::new(
                format!("Alias for {}", r.path),
                div()
                    .flex()
                    .gap(theme::SPACE_2)
                    .child(div().flex_1().child(self.alias_input.clone()))
                    .child(Button::new("alias-save", "Save").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save_alias(cx)))),
            )
        });
        div().flex().flex_col().gap(theme::SPACE_3).child(list).children(editor)
    }
}
