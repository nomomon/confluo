//! The one place that defines every key binding, its label, palette placement and availability.
//!
//! Menus (via GPUI key equivalents), tooltips (`keymap::tip`), the command palette (every entry is
//! a command), the `?` cheat sheet and the Settings Shortcuts page all read from this table, so a
//! binding changes in one line.

use gpui::{actions, Action, App, KeyBinding, SharedString};

use crate::state::cmd_ctx::{CmdCtx, PrAct};
use crate::ui::atoms::IconName;

actions!(
    git_viewer,
    [
        SelectNext,
        SelectPrev,
        OpenSelected,
        Dismiss,
        FocusSearch,
        Refresh,
        OpenInBrowser,
        CopyCheckout,
        ToggleStar,
        TabAuthored,
        TabReviewing,
        TabAll,
        NextFile,
        PrevFile,
        ToggleFile,
        OpenSettings,
        About,
        Minimize,
        Zoom,
        ToggleSidebar,
        ToggleDetail,
        CommandPalette,
        CloseDetail,
        GoInbox,
        GoStarred,
        HistoryBack,
        HistoryForward,
        CopyLink,
        PostComment,
        Approve,
        ToggleTheme,
        ToggleFileTree,
        ToggleRightPanel,
        QuickOpenFile,
        ToggleViewed,
        ToggleDiffView,
        GoProject1,
        GoProject2,
        GoProject3,
        GoProject4,
        GoProject5,
        GoProject6,
        GoProject7,
        NextPr,
        PrevPr,
        NextSection,
        PrevSection,
        RequestChanges,
        MergePr,
        ToggleDraft,
        ClosePr,
        CopyBranch,
        ShowShortcuts,
        NextUnresolved,
        PrevUnresolved,
        ThemeLight,
        ThemeDark,
        ThemeSystem,
        ZoomIn,
        ZoomOut,
        ZoomReset,
        ToggleWrap,
        GoToLine,
        ToggleFullScreen,
        OpenRecent,
        OpenLogs,
        DevReload,
        FindPrev
    ]
);

/// Where a binding is active.
#[derive(Clone, Copy)]
enum Ctx {
    /// Anywhere in the window, also while typing (bare character keys still skip text inputs).
    Any,
    /// Outside text inputs (letter keys, Cmd+arrows that move the caret in inputs).
    List,
}

/// Bare printable keys (`/`, `j`) must never fire inside a text input, whatever the entry says.
/// Cmd+Left / Cmd+Right move the caret in text inputs, so they also skip inputs.
fn predicate(ctx: Ctx, key: &str) -> Option<&'static str> {
    let bare = key.chars().count() == 1;
    let caret = matches!(key, "cmd-left" | "cmd-right");
    match ctx {
        Ctx::Any if !bare && !caret => Some("GitViewer"),
        _ => Some("GitViewer && !TextInput"),
    }
}

/// Cheat sheet and Settings sections, also the context a shortcut belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Global,
    List,
    Review,
    Diff,
    Palette,
}

impl Group {
    pub const ALL: [Group; 5] = [
        Group::Global,
        Group::List,
        Group::Review,
        Group::Diff,
        Group::Palette,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Group::Global => "Global",
            Group::List => "List",
            Group::Review => "Review",
            Group::Diff => "Diff",
            Group::Palette => "Palette",
        }
    }
}

/// Where an entry shows in the command palette.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pal {
    /// Not a palette command (j/k, Escape, ...).
    No,
    /// Context action for the open PR, listed under Suggested.
    Suggest,
    /// Under Go to.
    Go,
    /// Under Actions.
    Action,
}

/// One binding: the action, its keys (first is the label), label, sections and availability.
pub struct Entry {
    pub action: &'static str,
    pub keys: &'static [&'static str],
    pub label: &'static str,
    pub group: Group,
    pub pal: Pal,
    pub icon: IconName,
    /// The command makes sense now (palette rows only appear when true).
    pub available: fn(&CmdCtx) -> bool,
    pub build: fn() -> Box<dyn Action>,
}

impl Entry {
    /// Shown keys of the first binding, or empty.
    pub fn shortcut(&self) -> String {
        self.keys.first().map(|k| display(k)).unwrap_or_default()
    }
}

macro_rules! keymap {
    ($( $group:ident => { $( $action:ident [$($keys:literal),*] $ctx:ident $label:literal $pal:ident $icon:ident $avail:expr; )* } )*) => {
        /// Register every binding.
        pub fn bind_keys(cx: &mut App) {
            cx.bind_keys([ $($( $( KeyBinding::new($keys, $action, predicate(Ctx::$ctx, $keys)), )* )*)* ]);
        }

        /// Every entry, in display order.
        pub static ENTRIES: &[Entry] = &[ $($( Entry {
            action: stringify!($action),
            keys: &[$($keys),*],
            label: $label,
            group: Group::$group,
            pal: Pal::$pal,
            icon: IconName::$icon,
            available: $avail,
            build: || Box::new($action),
        }, )*)* ];
    };
}

fn always(_: &CmdCtx) -> bool {
    true
}

keymap! {
    Global => {
        CommandPalette ["cmd-k"] Any "Command palette" No Command always;
        ShowShortcuts ["?", "shift-/"] List "Keyboard shortcuts" Action Keyboard always;
        OpenSettings ["cmd-,"] Any "Settings" Action Settings always;
        Refresh ["cmd-r"] Any "Refresh" Action Refresh always;
        FocusSearch ["cmd-f", "/"] Any "Search the list (find in diff in Review)" Action Search always;
        ToggleSidebar ["cmd-b"] Any "Toggle sidebar" Action LayoutSidebar |c| !c.reviewing;
        ToggleDetail ["cmd-\\"] Any "Toggle preview" Action LayoutColumns |c| !c.reviewing;
        ToggleTheme ["cmd-shift-l"] Any "Theme: Cycle light, dark, system" Action Moon always;
        ThemeLight [] Any "Theme: Light" Action Bulb always;
        ThemeDark [] Any "Theme: Dark" Action Moon always;
        ThemeSystem [] Any "Theme: System" Action AdjustmentsHorizontal always;
        ZoomIn ["cmd-=", "cmd-+"] Any "View: Zoom in" Action Plus always;
        ZoomOut ["cmd--"] Any "View: Zoom out" Action CircleMinus always;
        ZoomReset ["cmd-0"] Any "View: Reset zoom" Action Search always;
        ToggleFullScreen ["ctrl-cmd-f"] Any "View: Toggle full screen" Action LayoutColumns always;
        OpenRecent ["ctrl-r"] Any "Open recent pull request..." Action Clock always;
        OpenLogs [] Any "Developer: Open logs folder" Action Folder always;
        DevReload [] Any "Developer: Reload" Action Refresh always;
        GoInbox ["cmd-1"] Any "Inbox" Go Inbox always;
        GoStarred ["cmd-2"] Any "Starred" Go Star always;
        GoProject1 ["cmd-3"] Any "Project 1" No Folder |c| c.projects > 0;
        GoProject2 ["cmd-4"] Any "Project 2" No Folder |c| c.projects > 1;
        GoProject3 ["cmd-5"] Any "Project 3" No Folder |c| c.projects > 2;
        GoProject4 ["cmd-6"] Any "Project 4" No Folder |c| c.projects > 3;
        GoProject5 ["cmd-7"] Any "Project 5" No Folder |c| c.projects > 4;
        GoProject6 ["cmd-8"] Any "Project 6" No Folder |c| c.projects > 5;
        GoProject7 ["cmd-9"] Any "Project 7" No Folder |c| c.projects > 6;
        HistoryBack ["cmd-[", "cmd-left"] Any "Back" Action ArrowLeft always;
        HistoryForward ["cmd-]", "cmd-right"] Any "Forward" Action ArrowRight always;
        CloseDetail ["cmd-w"] Any "Close (overlay, review, preview, window)" No X always;
        Dismiss ["escape"] Any "Close or clear" No X always;
        Minimize ["cmd-m"] Any "Minimize" No X always;
    }
    List => {
        SelectNext ["j"] List "Next row" No ChevronDown always;
        SelectPrev ["k"] List "Previous row" No ChevronUp always;
        NextPr ["cmd-down"] List "Next pull request" Action ChevronDown always;
        PrevPr ["cmd-up"] List "Previous pull request" Action ChevronUp always;
        NextSection ["alt-cmd-down"] List "Next section" Action ChevronDown always;
        PrevSection ["alt-cmd-up"] List "Previous section" Action ChevronUp always;
        OpenSelected ["enter"] List "Open in Review mode" Suggest Eye |c| c.pr_open && !c.reviewing;
        OpenInBrowser ["cmd-o", "o"] List "Open in browser" Suggest ExternalLink |c| c.pr_open;
        CopyCheckout ["c"] List "Copy checkout command" Suggest Terminal |c| c.pr_open;
        ToggleStar ["s"] List "Star / unstar" Suggest Star |c| c.pr_open;
        TabAuthored ["1"] List "Show authored" Action Filter |c| !c.reviewing;
        TabReviewing ["2"] List "Show reviewing" Action Filter |c| !c.reviewing;
        TabAll ["3"] List "Show all" Action Filter |c| !c.reviewing;
    }
    Review => {
        Approve ["cmd-shift-a"] Any "Approve" Suggest CircleCheck |c| c.allows(PrAct::Approve);
        RequestChanges ["cmd-shift-x"] Any "Request changes..." Suggest CircleX |c| c.allows(PrAct::RequestChanges);
        MergePr ["cmd-shift-m"] Any "Merge..." Suggest GitMerge |c| c.allows(PrAct::Merge);
        ToggleDraft ["cmd-shift-d"] Any "Toggle draft" Suggest GitPullRequestDraft |c| c.allows(PrAct::ToggleDraft);
        ClosePr [] Any "Close or reopen pull request..." Suggest GitPullRequestClosed |c| c.allows(PrAct::Close);
        CopyLink ["cmd-shift-c"] Any "Copy link" Suggest Link |c| c.pr_open;
        CopyBranch [] Any "Copy branch name" Suggest Copy |c| c.pr_open;
        PostComment ["cmd-enter"] Any "Post comment" No Message always;
        QuickOpenFile ["cmd-p"] Any "Go to file..." Suggest Search |c| c.reviewing;
        ToggleFileTree ["cmd-shift-e"] Any "Toggle file tree" Suggest LayoutSidebar |c| c.reviewing;
        ToggleRightPanel ["cmd-shift-i"] Any "Toggle right panel" Suggest LayoutSidebarRight |c| c.reviewing;
        ToggleDiffView ["cmd-shift-s"] Any "Unified / side by side" Suggest LayoutColumns |c| c.reviewing;
        NextUnresolved ["u"] List "Next unresolved thread" Suggest MessageCircle |c| c.reviewing;
        PrevUnresolved ["shift-u"] List "Previous unresolved thread" Suggest MessageCircle |c| c.reviewing;
    }
    Diff => {
        NextFile ["n", "]"] List "Next file" Suggest ChevronDown |c| c.diff;
        PrevFile ["p", "["] List "Previous file" Suggest ChevronUp |c| c.diff;
        ToggleViewed ["v"] List "Toggle viewed (next unviewed)" Suggest Eye |c| c.diff;
        ToggleFile ["x"] List "Collapse file" Action Fold |c| c.diff;
        ToggleWrap ["alt-z"] List "Toggle word wrap" Suggest LayoutRows |c| c.reviewing;
        GoToLine ["ctrl-g"] Any "Go to line..." Suggest FileText |c| c.file_view;
    }
}

/// Diff keys handled by the line cursor (`code_keys`, not palette commands).
pub const LINE_KEYS: [(&str, &str); 4] = [
    ("Move line cursor", "\u{2191} \u{2193}"),
    ("Extend selection", "\u{21e7} \u{2191}"),
    ("Comment on lines", "C"),
    ("Copy lines", "\u{2318} C"),
];

/// Keys of the find bar over the diff (Cmd+F in Review mode).
pub const FIND_KEYS: [(&str, &str); 3] = [
    ("Find in diff", "\u{2318} F"),
    ("Next / previous match", "\u{21a9}  \u{21e7} \u{21a9}"),
    ("Close find", "esc"),
];

/// Find bar bindings, active only while its field has focus.
pub fn bind_find_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("shift-enter", FindPrev, Some("FindBar"))]);
}

/// Keys inside the command palette.
pub const PALETTE_KEYS: [(&str, &str); 9] = [
    ("Move", "\u{2191} \u{2193}"),
    ("Run", "\u{21a9}"),
    ("Open pull request in browser", "\u{2318} \u{21a9}"),
    ("Close", "esc"),
    ("Actions only", ">"),
    ("Pull requests by number", "#"),
    ("Files of the pull request", "@"),
    ("Views and projects", "/"),
    ("Shortcuts", "?"),
];

/// Rows of a cheat sheet section: label and shown keys (all bindings joined).
pub fn group_rows(g: Group) -> Vec<(&'static str, Vec<String>)> {
    let mut rows: Vec<(&'static str, Vec<String>)> = ENTRIES
        .iter()
        .filter(|e| e.group == g && !e.keys.is_empty())
        .map(|e| {
            (
                e.label,
                e.keys
                    .iter()
                    .filter(|k| **k != "shift-/")
                    .map(|k| display(k))
                    .collect(),
            )
        })
        .collect();
    match g {
        Group::Diff => rows.extend(
            LINE_KEYS
                .iter()
                .chain(&FIND_KEYS)
                .map(|(l, k)| (*l, vec![k.to_string()])),
        ),
        Group::Palette => rows.extend(PALETTE_KEYS.iter().map(|(l, k)| (*l, vec![k.to_string()]))),
        _ => {}
    }
    rows
}

/// `cmd-shift-c` as shown on keys: `⌘ ⇧ C`.
pub fn display(keys: &str) -> String {
    let mut parts: Vec<&str> = keys.split('-').collect();
    // `cmd--` is Cmd and the minus key: the split leaves two empty parts.
    if parts.len() >= 2 && parts[parts.len() - 2..] == ["", ""] {
        parts.truncate(parts.len() - 2);
        parts.push("");
    }
    parts
        .into_iter()
        .map(|k| match k {
            "cmd" => "\u{2318}".to_string(),
            "shift" => "\u{21e7}".to_string(),
            "alt" => "\u{2325}".to_string(),
            "ctrl" => "\u{2303}".to_string(),
            "enter" => "\u{21a9}".to_string(),
            "up" => "\u{2191}".to_string(),
            "down" => "\u{2193}".to_string(),
            "left" => "\u{2190}".to_string(),
            "right" => "\u{2192}".to_string(),
            "escape" => "esc".to_string(),
            "" => "-".to_string(),
            k => k.to_uppercase(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn entry(action: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|e| e.action == action)
}

/// Shown keys of an action's first binding, e.g. `kbd::<Refresh>()` is `⌘ R`.
pub fn kbd<A: Action>() -> String {
    let name = A::name_for_type();
    let short = name.rsplit("::").next().unwrap_or(name);
    entry(short).map(Entry::shortcut).unwrap_or_default()
}

/// Tooltip text with the shortcut after a double space (rendered as `Kbd`).
pub fn tip<A: Action>(label: &str) -> SharedString {
    let k = kbd::<A>();
    if k.is_empty() {
        label.to_string().into()
    } else {
        format!("{label}  {k}").into()
    }
}

/// The `GoProjectN` action for the project at sidebar index `i` (Cmd+3..9), if any.
pub fn project_entry(i: usize) -> Option<&'static Entry> {
    ENTRIES
        .iter()
        .filter(|e| e.action.starts_with("GoProject"))
        .nth(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::cmd_ctx::Gate;
    use gv_engine::{EditCapabilities, ReviewCapabilities};

    #[test]
    fn labels() {
        assert_eq!(display("cmd-shift-c"), "\u{2318} \u{21e7} C");
        assert_eq!(display("cmd-\\"), "\u{2318} \\");
        assert_eq!(display("alt-cmd-down"), "\u{2325} \u{2318} \u{2193}");
        assert_eq!(kbd::<Refresh>(), "\u{2318} R");
        assert_eq!(kbd::<ToggleDetail>(), "\u{2318} \\");
        assert_eq!(kbd::<ToggleDraft>(), "\u{2318} \u{21e7} D");
        assert_eq!(display("cmd--"), "\u{2318} -");
        assert_eq!(display("cmd-="), "\u{2318} =");
        assert_eq!(kbd::<ToggleFullScreen>(), "\u{2303} \u{2318} F");
        assert_eq!(kbd::<ToggleWrap>(), "\u{2325} Z");
        assert_eq!(project_entry(0).map(|e| e.keys[0]), Some("cmd-3"));
        assert_eq!(project_entry(6).map(|e| e.keys[0]), Some("cmd-9"));
        assert!(project_entry(7).is_none());
    }

    #[test]
    fn no_duplicate_keys() {
        let mut seen = std::collections::HashSet::new();
        for e in ENTRIES {
            for k in e.keys {
                assert!(seen.insert(*k), "{k} bound twice");
            }
        }
    }

    #[test]
    fn bare_keys_never_fire_in_text_inputs() {
        assert_eq!(predicate(Ctx::Any, "/"), Some("GitViewer && !TextInput"));
        assert_eq!(predicate(Ctx::Any, "cmd-f"), Some("GitViewer"));
        assert_eq!(
            predicate(Ctx::List, "cmd-down"),
            Some("GitViewer && !TextInput")
        );
        // Cmd+Left / Cmd+Right go back / forward only outside inputs (inside they move the caret).
        assert_eq!(
            predicate(Ctx::Any, "cmd-left"),
            Some("GitViewer && !TextInput")
        );
        assert_eq!(predicate(Ctx::Any, "cmd-["), Some("GitViewer"));
        assert!(entry("HistoryBack").is_some_and(|e| e.keys.contains(&"cmd-left")));
        assert!(entry("HistoryForward").is_some_and(|e| e.keys.contains(&"cmd-right")));
    }

    fn avail(c: &CmdCtx) -> Vec<&'static str> {
        ENTRIES
            .iter()
            .filter(|e| e.pal != Pal::No && (e.available)(c))
            .map(|e| e.action)
            .collect()
    }

    #[test]
    fn availability_follows_context_and_capabilities() {
        let list = CmdCtx::default();
        let a = avail(&list);
        assert!(a.contains(&"Refresh") && a.contains(&"NextPr"));
        assert!(
            !a.contains(&"Approve")
                && !a.contains(&"ToggleViewed")
                && !a.contains(&"OpenInBrowser")
        );

        let own = CmdCtx {
            pr_open: true,
            reviewing: true,
            diff: true,
            projects: 2,
            file_view: false,
            gate: Some(Gate {
                review: ReviewCapabilities {
                    comment_review: true,
                    approve_blocked: Some("You can't approve your own PR".into()),
                    ..Default::default()
                },
                edit: EditCapabilities::editor(true),
            }),
        };
        let a = avail(&own);
        assert!(
            a.contains(&"ToggleDraft")
                && a.contains(&"ToggleViewed")
                && a.contains(&"ToggleDiffView")
        );
        assert!(
            !a.contains(&"Approve") && !a.contains(&"RequestChanges") && !a.contains(&"MergePr")
        );
        assert!(!a.contains(&"ToggleSidebar") && !a.contains(&"OpenSelected"));
        // Theme, zoom and developer commands are always in the palette; go to line needs the file viewer.
        assert!([
            "ThemeLight",
            "ThemeDark",
            "ThemeSystem",
            "ToggleTheme",
            "ZoomIn",
            "OpenLogs",
            "DevReload",
            "ToggleWrap"
        ]
        .iter()
        .all(|x| a.contains(x)));
        assert!(!a.contains(&"GoToLine"));
        assert!(avail(&list).contains(&"ThemeDark"));
        assert!(
            (project_entry(1).unwrap().available)(&own)
                && !(project_entry(2).unwrap().available)(&own)
        );
    }
}
