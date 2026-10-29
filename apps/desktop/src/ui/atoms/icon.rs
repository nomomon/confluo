//! SVG icon from the embedded Lucide set, tinted with one color.
//!
//! `Icon::new(IconName::ChevronDown).size(theme::SPACE_3).color(theme::TEXT_MUTED)`

use gpui::{prelude::*, svg, App, Hsla, Pixels, Window};

use crate::ui::theme;

/// The embedded set; not every icon is used by the app yet.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconName {
    Check,
    ChevronDown,
    ChevronRight,
    Copy,
    Ellipsis,
    ExternalLink,
    Folder,
    GitMerge,
    GitPullRequest,
    GitPullRequestClosed,
    GitPullRequestDraft,
    Inbox,
    Plus,
    Refresh,
    Search,
    Settings,
    Star,
    StarFilled,
    X,
}

impl IconName {
    #[cfg_attr(not(test), allow(dead_code))]
    pub const ALL: [IconName; 19] = [
        Self::Check,
        Self::ChevronDown,
        Self::ChevronRight,
        Self::Copy,
        Self::Ellipsis,
        Self::ExternalLink,
        Self::Folder,
        Self::GitMerge,
        Self::GitPullRequest,
        Self::GitPullRequestClosed,
        Self::GitPullRequestDraft,
        Self::Inbox,
        Self::Plus,
        Self::Refresh,
        Self::Search,
        Self::Settings,
        Self::Star,
        Self::StarFilled,
        Self::X,
    ];

    pub fn path(self) -> &'static str {
        match self {
            Self::Check => "icons/check.svg",
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::Copy => "icons/copy.svg",
            Self::Ellipsis => "icons/ellipsis.svg",
            Self::ExternalLink => "icons/external-link.svg",
            Self::Folder => "icons/folder.svg",
            Self::GitMerge => "icons/git-merge.svg",
            Self::GitPullRequest => "icons/git-pull-request.svg",
            Self::GitPullRequestClosed => "icons/git-pull-request-closed.svg",
            Self::GitPullRequestDraft => "icons/git-pull-request-draft.svg",
            Self::Inbox => "icons/inbox.svg",
            Self::Plus => "icons/plus.svg",
            Self::Refresh => "icons/refresh-cw.svg",
            Self::Search => "icons/search.svg",
            Self::Settings => "icons/settings.svg",
            Self::Star => "icons/star.svg",
            Self::StarFilled => "icons/star-filled.svg",
            Self::X => "icons/x.svg",
        }
    }
}

#[derive(IntoElement)]
pub struct Icon {
    name: IconName,
    size: Pixels,
    color: Hsla,
}

impl Icon {
    pub fn new(name: IconName) -> Self {
        Self { name, size: theme::ICON_MD, color: theme::TEXT_MUTED }
    }

    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        svg().path(self.name.path()).flex_shrink_0().size(self.size).text_color(self.color)
    }
}
