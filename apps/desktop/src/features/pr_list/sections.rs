//! Groups rows into rounded section cards (Needs you, Awaiting review, ...) in every view.

use gpui::{prelude::*, AnyElement, App, ClickEvent, Context};
use gv_engine::{ChangeRequest, ChangeRequestId};

use crate::app::GitViewer;
use crate::state::Section;
use crate::ui::molecules::ListSection;

impl GitViewer {
    /// Ids in on-screen order, skipping collapsed sections (for j/k).
    pub(crate) fn nav_order(&self, cx: &App) -> Vec<ChangeRequestId> {
        let visible = self.visible_items(cx);
        let collapsed = self.collapsed_sections();
        Section::ALL
            .into_iter()
            .filter(|s| !collapsed.contains(s))
            .flat_map(|s| visible.iter().filter(move |c| self.section_of(c) == s).map(|c| c.id.clone()).collect::<Vec<_>>())
            .collect()
    }

    /// Every view (Inbox, Starred, projects) groups rows into the same sections.
    pub(super) fn render_sections(&self, visible: &[&ChangeRequest], cx: &mut Context<Self>) -> Vec<AnyElement> {
        let fill = self.surfaces().card();
        // Provider badges only when the rows on screen mix providers.
        let shown = self.nav_order(cx);
        let mut kinds: Vec<_> = visible.iter().filter(|c| shown.contains(&c.id)).map(|c| c.id.repo.kind).collect();
        kinds.sort();
        kinds.dedup();
        let mixed = kinds.len() > 1;
        let collapsed = self.collapsed_sections();
        let mut out = vec![];
        for s in Section::ALL {
            let items: Vec<&&ChangeRequest> = visible.iter().filter(|c| self.section_of(c) == s).collect();
            if items.is_empty() {
                continue;
            }
            let open = !collapsed.contains(&s);
            let section = ListSection::new(format!("section-{s:?}"), fill)
                .header(s.title(), items.len())
                .open(open)
                .on_toggle(cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_section(s, cx)))
                .when(open, |sec| sec.children(items.iter().enumerate().map(|(i, c)| self.render_row(c, mixed, (i == 0, i + 1 == items.len()), cx).into_any_element())));
            out.push(section.into_any_element());
        }
        out
    }
}
