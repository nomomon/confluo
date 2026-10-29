//! Checkout card with a copyable CLI command.

use gpui::{div, prelude::*, ClickEvent, Context};
use gv_engine::{ChangeRequestId, ProviderKind};

use crate::app::GitViewer;
use crate::state::{Copied, DetailState};
use crate::ui::molecules::{Card, CopyField};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_checkout_card(&self, d: &DetailState, cx: &mut Context<Self>) -> impl IntoElement {
        Card::new(self.surfaces()).child(div().text_size(theme::TEXT_SM).font_weight(theme::WEIGHT_MEDIUM).child("Checkout")).child(
            CopyField::new("copy-cmd", checkout_command(&d.summary.id)).copied(self.copied == Some(Copied::Checkout))
                .on_copy(cx.listener(|this, _: &ClickEvent, _, cx| this.mark_copied(Copied::Checkout, cx))),
        )
    }
}

/// CLI command that checks out the PR locally with the provider's tool.
pub fn checkout_command(id: &ChangeRequestId) -> String {
    let r = &id.repo;
    let n = id.number;
    match r.kind {
        ProviderKind::GitHub if r.host == "github.com" => format!("gh pr checkout {n} --repo {}", r.path),
        ProviderKind::GitHub => format!("gh pr checkout {n} --repo {}/{}", r.host, r.path),
        ProviderKind::GitLab if r.host == "gitlab.com" => format!("glab mr checkout {n} -R {}", r.path),
        ProviderKind::GitLab => format!("glab mr checkout {n} -R {}/{}", r.host, r.path),
        ProviderKind::Azure => format!("az repos pr checkout --id {n}"),
    }
}

#[cfg(test)]
mod tests {
    use super::checkout_command;
    use gv_engine::{ChangeRequestId, ProviderKind, RepoRef};

    #[test]
    fn checkout_commands() {
        let id = ChangeRequestId { repo: RepoRef::new(ProviderKind::GitHub, "github.com", "o/r"), number: 4 };
        assert_eq!(checkout_command(&id), "gh pr checkout 4 --repo o/r");
        let id = ChangeRequestId { repo: RepoRef::new(ProviderKind::GitLab, "gitlab.com", "g/p"), number: 2 };
        assert_eq!(checkout_command(&id), "glab mr checkout 2 -R g/p");
    }
}
