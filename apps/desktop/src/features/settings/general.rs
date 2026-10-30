//! General: refresh interval, where PRs open and launch behavior.

use gpui::{div, prelude::*, Context};

use crate::app::GitViewer;
use crate::settings::{refresh_label, Launch, OpenIn, REFRESH_MINUTES};
use crate::ui::atoms::Switch;
use crate::ui::molecules::{Field, RadioGroup, Select, ToggleGroup};

impl GitViewer {
    pub(super) fn render_general(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let s = &self.settings;
        let refresh = REFRESH_MINUTES
            .iter()
            .fold(Select::new("refresh-interval"), |sel, m| sel.option(refresh_label(*m)))
            .selected(REFRESH_MINUTES.iter().position(|m| *m == s.refresh_minutes).unwrap_or(2))
            .on_change(cx.listener(|this, i: &usize, _, cx| {
                this.settings.refresh_minutes = REFRESH_MINUTES[*i];
                this.save_settings(cx)
            }));
        let open_in = ToggleGroup::new("open-in").item("Confluo", None, s.open_in == OpenIn::App).item("Browser", None, s.open_in == OpenIn::Browser).on_change(cx.listener(
            |this, i: &usize, _, cx| {
                this.settings.open_in = if *i == 0 { OpenIn::App } else { OpenIn::Browser };
                this.save_settings(cx)
            },
        ));
        let launch = Launch::ALL
            .iter()
            .fold(RadioGroup::new("launch"), |r, l| r.option(l.label()))
            .selected(Launch::ALL.iter().position(|l| *l == s.launch).unwrap_or(0))
            .on_change(cx.listener(|this, i: &usize, _, cx| {
                this.settings.launch = Launch::ALL[*i];
                this.save_settings(cx)
            }));
        let hints = Switch::new("shortcut-hints", s.shortcut_hints).on_change(cx.listener(|this, on: &bool, _, cx| {
            this.settings.shortcut_hints = *on;
            this.save_settings(cx)
        }));
        div().child(self.settings_group(
            None,
            vec![
                Field::new("Refresh", refresh).description("How often pull requests are fetched in the background").horizontal().into_any_element(),
                Field::new("Open pull requests in", open_in).description("Clicking a row opens the detail pane or the web page").horizontal().into_any_element(),
                Field::new("At launch", launch).description("Start in the Inbox or the project you had open").horizontal().into_any_element(),
                Field::new("Show shortcut hints when holding \u{2318}", hints).description("Hold Command alone to see the shortcuts of visible controls").horizontal().into_any_element(),
            ],
        ))
    }
}
