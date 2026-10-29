//! SVG icon from the embedded Tabler set (outline, stroke 1.75), tinted with one color.
//!
//! `Icon::new(IconName::ChevronDown).size(theme::SPACE_3).color(theme::text_muted())`

use gpui::{prelude::*, svg, App, Hsla, Pixels, Window};

use crate::ui::theme;

/// The embedded set; not every icon is used by the app yet.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconName {
    ArrowLeft,
    Check,
    ChevronDown,
    ChevronRight,
    ChevronUp,
    Copy,
    Ellipsis,
    ExternalLink,
    File,
    FileCode,
    FileText,
    Fold,
    Folder,
    GitMerge,
    GitPullRequest,
    GitPullRequestClosed,
    GitPullRequestDraft,
    Inbox,
    LayoutColumns,
    LayoutRows,
    LayoutSidebar,
    Link,
    Message,
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
    pub const ALL: [IconName; 30] = [
        Self::ArrowLeft,
        Self::Check,
        Self::ChevronDown,
        Self::ChevronRight,
        Self::ChevronUp,
        Self::Copy,
        Self::Ellipsis,
        Self::ExternalLink,
        Self::File,
        Self::FileCode,
        Self::FileText,
        Self::Fold,
        Self::Folder,
        Self::GitMerge,
        Self::GitPullRequest,
        Self::GitPullRequestClosed,
        Self::GitPullRequestDraft,
        Self::Inbox,
        Self::LayoutColumns,
        Self::LayoutRows,
        Self::LayoutSidebar,
        Self::Link,
        Self::Message,
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
            Self::ArrowLeft => "icons/arrow-left.svg",
            Self::Check => "icons/check.svg",
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::ChevronUp => "icons/chevron-up.svg",
            Self::Copy => "icons/copy.svg",
            Self::Ellipsis => "icons/dots.svg",
            Self::ExternalLink => "icons/external-link.svg",
            Self::File => "icons/file.svg",
            Self::FileCode => "icons/file-code.svg",
            Self::FileText => "icons/file-text.svg",
            Self::Fold => "icons/fold.svg",
            Self::Folder => "icons/folder.svg",
            Self::GitMerge => "icons/git-merge.svg",
            Self::GitPullRequest => "icons/git-pull-request.svg",
            Self::GitPullRequestClosed => "icons/git-pull-request-closed.svg",
            Self::GitPullRequestDraft => "icons/git-pull-request-draft.svg",
            Self::Inbox => "icons/inbox.svg",
            Self::LayoutColumns => "icons/layout-columns.svg",
            Self::LayoutRows => "icons/layout-rows.svg",
            Self::LayoutSidebar => "icons/layout-sidebar.svg",
            Self::Link => "icons/link.svg",
            Self::Message => "icons/message.svg",
            Self::Plus => "icons/plus.svg",
            Self::Refresh => "icons/refresh.svg",
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
        Self { name, size: theme::ICON_MD, color: theme::text_muted() }
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
