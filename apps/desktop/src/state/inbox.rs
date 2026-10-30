//! Inbox grouping: section of each PR (rules in core `classify`) and section collapse state.

use gpui::Context;
use gv_engine::ChangeRequest;

use crate::app::GitViewer;

pub use gv_engine::{classify, Section};

impl GitViewer {
    pub(crate) fn section_of(&self, c: &ChangeRequest) -> Section {
        classify(
            c,
            self.engine.me(c.id.repo.kind, &c.id.repo.host).as_deref(),
        )
    }

    /// Key of the current view for remembered collapse state: Inbox, Starred or a project.
    fn view_key(&self) -> String {
        match (self.selected, self.starred_only) {
            (_, true) => "starred".into(),
            (Some(id), _) => format!("project:{id}"),
            (None, false) => "inbox".into(),
        }
    }

    /// Collapsed sections of the current view; defaults until the user toggles one there.
    pub(crate) fn collapsed_sections(&self) -> Vec<Section> {
        self.settings
            .collapsed_sections
            .get(&self.view_key())
            .cloned()
            .unwrap_or_else(|| {
                Section::ALL
                    .into_iter()
                    .filter(|s| s.collapsed_by_default())
                    .collect()
            })
    }

    /// Collapse or open a section in the current view; remembered per view in settings.json.
    pub(crate) fn toggle_section(&mut self, s: Section, cx: &mut Context<Self>) {
        let mut v = self.collapsed_sections();
        match v.iter().position(|x| *x == s) {
            Some(i) => {
                v.remove(i);
            }
            None => v.push(s),
        }
        self.settings.collapsed_sections.insert(self.view_key(), v);
        self.save_settings(cx);
    }
}
