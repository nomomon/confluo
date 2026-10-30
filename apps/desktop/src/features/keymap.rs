//! The one place that defines every key binding and its label.
//!
//! Menus (via GPUI key equivalents), tooltips (`keymap::tip`) and the Settings Shortcuts page
//! all read from this table, so a binding changes in one line.

use gpui::{actions, Action, App, KeyBinding, SharedString};

actions!(
    git_viewer,
    [
        SelectNext, SelectPrev, OpenSelected, Dismiss, FocusSearch, Refresh, OpenInBrowser, CopyCheckout, ToggleStar, TabAuthored, TabReviewing, TabAll, NextFile, PrevFile,
        ToggleFile, OpenSettings, About, Minimize, Zoom, ToggleSidebar, ToggleDetail, CommandPalette, CloseDetail, GoInbox, GoStarred, GoProject, HistoryBack, HistoryForward,
        CopyLink, PostComment, Approve, ToggleTheme
    ]
);

/// Where a binding is active.
#[derive(Clone, Copy)]
enum Ctx {
    /// Anywhere in the window, also while typing.
    Any,
    /// Outside text inputs (letter keys).
    List,
}

impl Ctx {
    fn predicate(self) -> Option<&'static str> {
        match self {
            Ctx::Any => Some("GitViewer"),
            Ctx::List => Some("GitViewer && !TextInput"),
        }
    }
}

/// One row of the Shortcuts page: the action, its keys (first is the label) and a description.
pub struct Entry {
    pub action: &'static str,
    pub keys: &'static [&'static str],
    pub label: &'static str,
}

macro_rules! keymap {
    ($( $group:literal => { $( $action:ident [$($keys:literal),+] $ctx:ident $label:literal; )* } )*) => {
        /// Register every binding.
        pub fn bind_keys(cx: &mut App) {
            cx.bind_keys([ $($( $( KeyBinding::new($keys, $action, Ctx::$ctx.predicate()), )+ )*)* ]);
        }

        /// Bindings grouped for the Shortcuts page.
        pub const GROUPS: &[(&str, &[Entry])] = &[ $( ($group, &[ $( Entry { action: stringify!($action), keys: &[$($keys),+], label: $label }, )* ]), )* ];
    };
}

keymap! {
    "General" => {
        CommandPalette ["cmd-k"] Any "Command palette";
        OpenSettings ["cmd-,"] Any "Settings";
        Refresh ["cmd-r"] Any "Refresh";
        FocusSearch ["cmd-f", "/"] Any "Search";
        ToggleSidebar ["cmd-b"] Any "Toggle sidebar";
        ToggleDetail ["cmd-\\", "cmd-shift-d"] Any "Toggle detail pane";
        ToggleTheme ["cmd-shift-l"] Any "Toggle light / dark";
        Dismiss ["escape"] Any "Close or clear";
        Minimize ["cmd-m"] Any "Minimize";
    }
    "Go" => {
        GoInbox ["cmd-1"] Any "Inbox";
        GoStarred ["cmd-2"] Any "Starred";
        GoProject ["cmd-3"] Any "First project";
        HistoryBack ["cmd-["] Any "Back to previous PR";
        HistoryForward ["cmd-]"] Any "Forward";
    }
    "List" => {
        SelectNext ["j"] List "Next";
        SelectPrev ["k"] List "Previous";
        OpenSelected ["enter"] List "Open";
        OpenInBrowser ["cmd-o", "o"] List "Open in browser";
        CopyCheckout ["c"] List "Copy checkout";
        ToggleStar ["s"] List "Star";
        TabAuthored ["1"] List "Authored";
        TabReviewing ["2"] List "Reviewing";
        TabAll ["3"] List "All";
    }
    "Pull request" => {
        CloseDetail ["cmd-w"] Any "Close detail (or window)";
        CopyLink ["cmd-shift-c"] Any "Copy link";
        PostComment ["cmd-enter"] Any "Post comment";
        Approve ["cmd-shift-a"] Any "Approve";
    }
    "Code tab" => {
        NextFile ["n", "]"] List "Next file";
        PrevFile ["p", "["] List "Previous file";
        ToggleFile ["x"] List "Collapse file";
    }
}

/// `cmd-shift-c` as shown on keys: `⌘ ⇧ C`.
pub fn display(keys: &str) -> String {
    keys.split('-')
        .map(|k| match k {
            "cmd" => "\u{2318}".to_string(),
            "shift" => "\u{21e7}".to_string(),
            "alt" => "\u{2325}".to_string(),
            "ctrl" => "\u{2303}".to_string(),
            "enter" => "\u{21a9}".to_string(),
            "escape" => "esc".to_string(),
            "" => "-".to_string(),
            k => k.to_uppercase(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn entry(action: &str) -> Option<&'static Entry> {
    GROUPS.iter().flat_map(|(_, e)| e.iter()).find(|e| e.action == action)
}

/// Shown keys of an action's first binding, e.g. `kbd::<Refresh>()` is `⌘ R`.
pub fn kbd<A: Action>() -> String {
    let name = A::name_for_type();
    let short = name.rsplit("::").next().unwrap_or(name);
    entry(short).map(|e| display(e.keys[0])).unwrap_or_default()
}

/// Tooltip text with the shortcut after a double space (rendered as `Kbd`).
pub fn tip<A: Action>(label: &str) -> SharedString {
    let k = kbd::<A>();
    if k.is_empty() { label.to_string().into() } else { format!("{label}  {k}").into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(display("cmd-shift-c"), "\u{2318} \u{21e7} C");
        assert_eq!(display("cmd-\\"), "\u{2318} \\");
        assert_eq!(kbd::<Refresh>(), "\u{2318} R");
        assert_eq!(kbd::<ToggleDetail>(), "\u{2318} \\");
    }

    #[test]
    fn no_duplicate_keys_in_a_context() {
        let mut seen = std::collections::HashSet::new();
        for (_, es) in GROUPS {
            for e in es.iter() {
                for k in e.keys {
                    assert!(seen.insert(*k), "{k} bound twice");
                }
            }
        }
    }
}
