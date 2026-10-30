//! Groups rows into rounded section cards (Needs you, Awaiting review, ...) in every view.

use gpui::{prelude::*, AnyElement, App, ClickEvent, Context};
use gv_engine::{ChangeRequest, ChangeRequestId};

use crate::app::GitViewer;
use crate::state::Section;
use crate::ui::molecules::ListSection;

impl GitViewer {
    /// Sections on screen with their ids, collapsed and empty ones skipped, search applied.
    pub(crate) fn nav_sections(&self, cx: &App) -> Vec<(Section, Vec<ChangeRequestId>)> {
        let rows: Vec<_> = self.visible_items(cx).into_iter().map(|c| (c.id.clone(), self.section_of(c))).collect();
        crate::state::pr_nav::visible_sections(&rows, &self.collapsed_sections())
    }

    /// Ids in on-screen order, skipping collapsed sections (for j/k and Cmd+Up/Down).
    pub(crate) fn nav_order(&self, cx: &App) -> Vec<ChangeRequestId> {
        self.nav_sections(cx).into_iter().flat_map(|(_, ids)| ids).collect()
    }

    /// Every view (Inbox, Starred, projects) groups rows into the same sections.
    pub(super) fn render_sections(&self, visible: &[&ChangeRequest], cx: &mut Context<Self>) -> Vec<AnyElement> {
        let fill = self.surfaces().card();
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
                .when(open, |sec| sec.children(items.iter().map(|c| self.render_row(c, cx).into_any_element())));
            out.push(section.into_any_element());
        }
        out
    }
}
