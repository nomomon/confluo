//! Empty list: shows which hosts are signed in, or how to sign in.

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context};

use crate::app::GitViewer;
use crate::state::auth_command;
use crate::ui::atoms::Button;
use crate::ui::molecules::EmptyState;
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_empty(&self, cx: &mut Context<Self>) -> AnyElement {
        let add = || Button::new("empty-add-project", "Add project").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.open_add_project(cx)));
        let Some(hosts) = &self.hosts else {
            return EmptyState::new("Checking connections...", "Looking for signed-in GitHub and GitLab accounts").into_any_element();
        };
        let connected = self.connected_hosts();
        if connected.is_empty() {
            let mut cmds: Vec<&str> = hosts.iter().map(|h| auth_command(h.kind)).collect();
            cmds.dedup();
            let hint = format!("Run `{}`, then refresh", cmds.join("` or `"));
            return EmptyState::new("No accounts connected", hint)
                .child(
                    div()
                        .flex()
                        .gap(theme::SPACE_2)
                        .child(Button::new("empty-retry", "Retry").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.retry(cx))))
                        .child(add()),
                )
                .into_any_element();
        }
        let title = if self.refreshing { "Fetching pull requests..." } else { "No open pull requests" };
        let who: Vec<String> = connected.iter().map(|h| format!("{} as @{}", h.host, h.user.as_deref().unwrap_or_default())).collect();
        EmptyState::new(title, format!("Connected: {}", who.join(", "))).child(add()).into_any_element()
    }
}
