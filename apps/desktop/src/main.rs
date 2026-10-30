//! Confluo desktop app (GPUI).

mod app;
mod assets;
mod dev;
mod features;
mod fonts;
mod settings;
mod state;
mod ui;

use gpui::{
    actions, point, px, size, App, AppContext, Application, Bounds, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType, TitlebarOptions,
    WindowBounds, WindowOptions,
};
use std::sync::Arc;

actions!(git_viewer, [Quit]);

fn main() {
    let engine = match gv_engine::Engine::open_default() {
        Ok(e) => Arc::new(e),
        Err(e) => {
            eprintln!("confluo: cannot open cache: {e:#}");
            std::process::exit(1);
        }
    };
    let mut settings = settings::Settings::load();
    dev::apply_overrides(&mut settings);
    ui::motion::init();

    Application::new().with_assets(assets::Assets).run(move |cx: &mut App| {
        fonts::register(cx);
        ui::theme::set_typography(settings.typography());
        ui::molecules::text_input::bind_keys(cx);
        ui::molecules::command_palette::bind_keys(cx);
        features::keyboard::bind_keys(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.on_window_closed(|cx| cx.quit()).detach();
        set_menus(cx);

        let bounds = Bounds::centered(None, dev::window_size().unwrap_or(size(px(1280.), px(820.))), cx);
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Confluo".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(14.), px(14.))),
            }),
            window_background: settings.appearance.to_gpui(),
            window_min_size: Some(size(px(820.), px(520.))),
            ..Default::default()
        };
        cx.open_window(opts, |window, cx| cx.new(|cx| app::GitViewer::new(engine.clone(), settings.clone(), window, cx)))
            .expect("open window");
        cx.activate(true);
    });
}

fn set_menus(cx: &mut App) {
    use features::keymap::*;
    use ui::molecules::text_input as ti;
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
                MenuItem::action("Quit Confluo", Quit),
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
                MenuItem::action("Command Palette...", CommandPalette),
                MenuItem::action("Find", FocusSearch),
                MenuItem::action("Refresh", Refresh),
                MenuItem::separator(),
                MenuItem::action("Toggle Light / Dark Theme", ToggleTheme),
            ],
        },
        Menu {
            name: "Go".into(),
            items: vec![
                MenuItem::action("Inbox", GoInbox),
                MenuItem::action("Starred", GoStarred),
                MenuItem::action("First Project", GoProject),
                MenuItem::separator(),
                MenuItem::action("Back", HistoryBack),
                MenuItem::action("Forward", HistoryForward),
            ],
        },
        Menu {
            name: "Pull Request".into(),
            items: vec![
                MenuItem::action("Open in Browser", OpenInBrowser),
                MenuItem::action("Copy Link", CopyLink),
                MenuItem::separator(),
                MenuItem::action("Open Review", OpenSelected),
                MenuItem::action("Go to File...", QuickOpenFile),
                MenuItem::action("Next File", NextFile),
                MenuItem::action("Previous File", PrevFile),
                MenuItem::action("Toggle Viewed", ToggleViewed),
                MenuItem::separator(),
                MenuItem::action("Approve", Approve),
                MenuItem::action("Post Comment", PostComment),
                MenuItem::separator(),
                MenuItem::action("Close", CloseDetail),
            ],
        },
        Menu { name: "Window".into(), items: vec![MenuItem::action("Minimize", Minimize), MenuItem::action("Zoom", Zoom)] },
    ]);
}
