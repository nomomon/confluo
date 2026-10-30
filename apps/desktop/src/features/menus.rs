//! App menu bar. Key equivalents come from the keymap bindings; the Go menu lists the projects
//! (rebuilt when they change), so Cmd+3..9 show beside their names. View > Appearance marks the
//! current theme with a check (GPUI menus have no checked state, so it is part of the title; the
//! menus are rebuilt when the theme changes).

use gpui::{App, Menu, MenuItem, OsAction, SystemMenuType};

use crate::settings::ThemeMode;

/// "✓ Dark" for the current theme, the plain label (indented to line up) otherwise.
pub fn theme_title(mode: ThemeMode, current: ThemeMode) -> String {
    let mark = if mode == current { "\u{2713} " } else { "    " };
    format!("{mark}{}", mode.label())
}

use super::keymap::*;
use crate::ui::molecules::text_input as ti;

/// Install the menus; `projects` are the sidebar project names in order.
/// `draft`: state of the open PR, for the draft item's label.
pub fn set_menus(cx: &mut App, projects: &[String], theme: ThemeMode, draft: Option<bool>) {
    let mut go = vec![
        MenuItem::action("Inbox", GoInbox),
        MenuItem::action("Starred", GoStarred),
    ];
    let gos: [fn(String) -> MenuItem; 7] = [
        |n| MenuItem::action(n, GoProject1),
        |n| MenuItem::action(n, GoProject2),
        |n| MenuItem::action(n, GoProject3),
        |n| MenuItem::action(n, GoProject4),
        |n| MenuItem::action(n, GoProject5),
        |n| MenuItem::action(n, GoProject6),
        |n| MenuItem::action(n, GoProject7),
    ];
    if !projects.is_empty() {
        go.push(MenuItem::separator());
        go.extend(projects.iter().zip(gos).map(|(name, f)| f(name.clone())));
    }
    go.extend([
        MenuItem::separator(),
        MenuItem::action("Next Pull Request", NextPr),
        MenuItem::action("Previous Pull Request", PrevPr),
        MenuItem::action("Next Section", NextSection),
        MenuItem::action("Previous Section", PrevSection),
        MenuItem::separator(),
        MenuItem::action("Back", HistoryBack),
        MenuItem::action("Forward", HistoryForward),
        MenuItem::separator(),
        MenuItem::action("Open Recent...", OpenRecent),
        MenuItem::action("Go to Line...", GoToLine),
    ]);
    let appearance = Menu {
        name: "Appearance".into(),
        items: vec![
            MenuItem::action(theme_title(ThemeMode::Light, theme), ThemeLight),
            MenuItem::action(theme_title(ThemeMode::Dark, theme), ThemeDark),
            MenuItem::action(theme_title(ThemeMode::System, theme), ThemeSystem),
            MenuItem::separator(),
            MenuItem::action("Cycle Theme", ToggleTheme),
        ],
    };
    cx.set_menus(vec![
        Menu {
            name: "Confluo".into(),
            items: vec![
                MenuItem::action("About Confluo", About),
                MenuItem::separator(),
                MenuItem::action("Settings...", OpenSettings),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit Confluo", crate::Quit),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Cut", ti::Cut, OsAction::Cut),
                MenuItem::os_action("Copy", ti::Copy, OsAction::Copy),
                MenuItem::os_action("Paste", ti::Paste, OsAction::Paste),
                MenuItem::separator(),
                MenuItem::os_action("Select All", ti::SelectAll, OsAction::SelectAll),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Toggle Preview", ToggleDetail),
                MenuItem::action("Toggle File Tree", ToggleFileTree),
                MenuItem::action("Toggle Right Panel", ToggleRightPanel),
                MenuItem::action("Unified / Side by Side", ToggleDiffView),
                MenuItem::separator(),
                MenuItem::action("Toggle Word Wrap", ToggleWrap),
                MenuItem::separator(),
                MenuItem::action("Command Palette...", CommandPalette),
                MenuItem::action("Find", FocusSearch),
                MenuItem::action("Refresh", Refresh),
                MenuItem::separator(),
                MenuItem::submenu(appearance),
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::action("Actual Size", ZoomReset),
                MenuItem::separator(),
                MenuItem::action("Toggle Full Screen", ToggleFullScreen),
            ],
        },
        Menu {
            name: "Go".into(),
            items: go,
        },
        Menu {
            name: "Pull Request".into(),
            items: vec![
                MenuItem::action("Open Review", OpenSelected),
                MenuItem::action("Open in Browser", OpenInBrowser),
                MenuItem::separator(),
                MenuItem::action("Approve", Approve),
                MenuItem::action("Request Changes...", RequestChanges),
                MenuItem::action("Merge...", MergePr),
                MenuItem::action(crate::state::cmd_ctx::draft_label(draft), ToggleDraft),
                MenuItem::action("Close or Reopen...", ClosePr),
                MenuItem::separator(),
                MenuItem::action("Copy Link", CopyLink),
                MenuItem::action("Copy Branch Name", CopyBranch),
                MenuItem::action("Copy Checkout Command", CopyCheckout),
                MenuItem::separator(),
                MenuItem::action("Go to File...", QuickOpenFile),
                MenuItem::action("Next File", NextFile),
                MenuItem::action("Previous File", PrevFile),
                MenuItem::action("Toggle Viewed", ToggleViewed),
                MenuItem::separator(),
                MenuItem::action("Post Comment", PostComment),
                MenuItem::action("Close", CloseDetail),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Minimize", Minimize),
                MenuItem::action("Zoom", Zoom),
            ],
        },
        Menu {
            name: "Help".into(),
            items: vec![
                MenuItem::action("Keyboard Shortcuts", ShowShortcuts),
                MenuItem::separator(),
                MenuItem::action("Open Logs Folder", OpenLogs),
                MenuItem::action("Reload", DevReload),
            ],
        },
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_marks_the_current_theme() {
        assert_eq!(
            theme_title(ThemeMode::Dark, ThemeMode::Dark),
            "\u{2713} Dark"
        );
        assert_eq!(theme_title(ThemeMode::Light, ThemeMode::Dark), "    Light");
    }
}
