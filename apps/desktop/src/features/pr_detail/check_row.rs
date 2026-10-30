//! One check row: status, name, stage, duration, finish time, open logs; failed checks
//! expand to the tail of their log.

use chrono::Utc;
use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};
use gv_engine::{format_duration, CheckRun};

use super::checks::status_icon;
use crate::app::GitViewer;
use crate::features::shared::relative_time;
use crate::state::{DetailState, LogState};
use crate::ui::atoms::{ErrorText, Icon, IconName, ListItem, Spinner, Button, ButtonVariant};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_check_row(&self, d: &DetailState, r: &CheckRun, cx: &mut Context<Self>) -> AnyElement {
        let key = if r.id.is_empty() { r.name.clone() } else { r.id.clone() };
        let open = d.ui.open_checks.contains(&r.id) && r.has_log;
        let meta = |s: String| div().flex_shrink_0().text_size(theme::text_xs()).text_color(theme::text_subtle()).child(s);
        let logs = r.url.clone().map(|url| {
            Button::new(gpui::SharedString::from(format!("check-open-{key}")), "").icon(IconName::ExternalLink).variant(ButtonVariant::Ghost).tooltip("Open logs").on_click(move |_: &ClickEvent, _, cx| {
                cx.stop_propagation();
                cx.open_url(&url)
            })
        });
        let id = r.id.clone();
        let row = div()
            .id(gpui::SharedString::from(format!("check-{key}")))
            .group(gpui::SharedString::from(format!("check-g-{key}")))
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .pl(theme::row_pad_x())
            .pr(theme::SPACE_1)
            .min_h(theme::control_h())
            .text_size(theme::text_sm())
            .list_item(open)
            .child(status_icon(&key, r.status))
            .child(div().flex_shrink_0().max_w(gpui::relative(0.5)).truncate().child(r.name.clone()))
            .children(r.stage.clone().map(|s| div().min_w(theme::SPACE_0).truncate().text_size(theme::text_xs()).text_color(theme::text_subtle()).child(s)))
            .child(div().flex_1())
            .children(r.duration_secs(Utc::now()).filter(|s| *s > 0).map(|s| meta(format_duration(s))))
            .children(r.finished_at.map(|t| meta(relative_time(t))))
            .when(r.has_log, |el| el.child(Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight }).size(theme::ICON_SM).color(theme::text_subtle())))
            .children(logs)
            .when(r.has_log, |el| el.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_check(id.clone(), cx))));
        if !open {
            return row.into_any_element();
        }
        div().flex().flex_col().child(row).child(self.render_log(&key, d.ui.logs.get(&r.id))).into_any_element()
    }

    fn render_log(&self, key: &str, log: Option<&LogState>) -> AnyElement {
        let body: AnyElement = match log {
            None | Some(LogState::Loading) => div()
                .flex()
                .items_center()
                .gap(theme::SPACE_2)
                .text_color(theme::text_muted())
                .child(Spinner::new(format!("log-{key}")).size(theme::ICON_SM))
                .child("Loading log...")
                .into_any_element(),
            Some(LogState::Failed(e)) => ErrorText::new(format!("Couldn't load the log: {e}")).into_any_element(),
            Some(LogState::Lines(l)) if l.is_empty() => div().text_color(theme::text_muted()).child("No log output here; open the logs for details").into_any_element(),
            Some(LogState::Lines(l)) => div()
                .flex()
                .flex_col()
                .font_family(theme::mono_font())
                .text_color(theme::text_muted())
                .children(l.iter().map(|line| div().whitespace_nowrap().overflow_hidden().child(line.clone())))
                .into_any_element(),
        };
        div()
            .mx(theme::row_pad_x())
            .mt(theme::SPACE_1)
            .mb(theme::SPACE_2)
            .p(theme::SPACE_2P5)
            .rounded(theme::RADIUS_MD)
            .bg(theme::surface())
            .text_size(theme::text_xs())
            .overflow_hidden()
            .child(body)
            .into_any_element()
    }
}
