//! The open PR: the triage preview and Review mode (top bar, file tree, diff, right panel).

mod activity;
mod check_row;
mod checkout;
mod checks;
mod ci;
mod code;
mod comment;
mod comment_actions;
mod comment_links;
mod comment_menu;
mod comment_run;
mod comment_toolbar;
mod commits;
mod description;
mod edit_dialogs;
mod edit_menu;
mod header;
mod people_group;
mod preview;
mod preview_reviewers;
mod properties;
mod quick_open;
mod reactions;
mod review;
mod right_panel;
mod stats;
mod thread_view;

pub(crate) use checkout::checkout_command;

use crate::app::GitViewer;
use crate::ui::molecules::markdown::{image_urls, Markdown};

impl GitViewer {
    /// Markdown with whichever of its remote images have loaded.
    pub(super) fn markdown(&self, id: impl Into<gpui::SharedString>, src: &str) -> Markdown {
        let src = crate::dev::markdown_override().unwrap_or(src);
        let images: std::collections::HashMap<_, _> = image_urls(src)
            .into_iter()
            .filter_map(|u| self.avatar(Some(&u)).map(|i| (u, i)))
            .collect();
        Markdown::new(id, src).images(std::rc::Rc::new(images))
    }
}
