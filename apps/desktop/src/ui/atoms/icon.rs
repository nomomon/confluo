//! SVG icon from the embedded Tabler set (outline, stroke 1.75), tinted with one color.
//!
//! `Icon::new(IconName::ChevronDown).size(theme::icon_sm()).color(theme::text_muted())`

use gpui::{prelude::*, svg, App, Hsla, Pixels, Window};

use crate::ui::theme;

macro_rules! icon_set {
    ($($variant:ident => $file:literal,)*) => {
        /// The embedded set; not every icon is used by the app yet.
        #[allow(dead_code)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum IconName { $($variant,)* }

        impl IconName {
            #[cfg_attr(not(test), allow(dead_code))]
            pub const ALL: &'static [IconName] = &[$(Self::$variant,)*];

            pub fn path(self) -> &'static str {
                match self { $(Self::$variant => concat!("icons/", $file, ".svg"),)* }
            }
        }
    };
}

icon_set! {
    AdjustmentsHorizontal => "adjustments",
    AlertCircle => "alert-circle",
    AlertTriangle => "alert-triangle",
    ArrowBackUp => "arrow-back-up",
    ArrowLeft => "arrow-left",
    ArrowRight => "arrow-right",
    BrandAzure => "brand-azure",
    BrandGithub => "brand-github",
    BrandGitlab => "brand-gitlab",
    Brush => "brush",
    Bulb => "bulb",
    Check => "check",
    ChevronDown => "chevron-down",
    ChevronLeft => "chevron-left",
    ChevronRight => "chevron-right",
    ChevronUp => "chevron-up",
    CircleCheck => "circle-check",
    CircleCheckFilled => "circle-check-filled",
    CircleDashed => "circle-dashed",
    CircleMinus => "circle-minus",
    CircleX => "circle-x",
    Clock => "clock",
    Code => "code",
    Command => "command",
    Copy => "copy",
    Download => "download",
    CornerUpLeft => "corner-up-left",
    Ellipsis => "dots",
    ExternalLink => "external-link",
    Eye => "eye",
    EyeOff => "eye-off",
    File => "file",
    FileCode => "file-code",
    FileText => "file-text",
    Filter => "filter",
    Fold => "fold",
    Folder => "folder",
    GitCommit => "git-commit",
    GitMerge => "git-merge",
    GitPullRequest => "git-pull-request",
    GitPullRequestClosed => "git-pull-request-closed",
    GitPullRequestDraft => "git-pull-request-draft",
    Inbox => "inbox",
    InfoCircle => "info-circle",
    Keyboard => "keyboard",
    LayoutColumns => "layout-columns",
    LayoutRows => "layout-rows",
    LayoutSidebar => "layout-sidebar",
    LayoutSidebarRight => "layout-sidebar-right",
    Link => "link",
    Loader => "loader-2",
    Message => "message",
    MessageCircle => "message-circle",
    Moon => "moon",
    MoodPlus => "mood-plus",
    Pencil => "pencil",
    Photo => "photo",
    Plus => "plus",
    Refresh => "refresh",
    Search => "search",
    Selector => "selector",
    Settings => "settings",
    Star => "star",
    StarFilled => "star-filled",
    Tag => "tag",
    Terminal => "terminal",
    Trash => "trash",
    User => "user",
    Users => "users",
    X => "x",
}

#[derive(IntoElement)]
pub struct Icon {
    name: IconName,
    size: Pixels,
    color: Hsla,
}

impl Icon {
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            size: theme::icon_md(),
            color: theme::text_muted(),
        }
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
        svg()
            .path(self.name.path())
            .flex_shrink_0()
            .size(self.size)
            .text_color(self.color)
    }
}
