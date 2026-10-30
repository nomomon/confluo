//! Confluo desktop app (GPUI).

mod app;
mod assets;
mod crash;
mod dev;
mod features;
mod fonts;
mod platform;
mod settings;
mod state;
mod ui;

use gpui::{
    actions, point, px, size, App, AppContext, Application, Bounds, KeyBinding, TitlebarOptions,
    WindowBounds, WindowOptions,
};
use std::sync::Arc;

actions!(git_viewer, [Quit]);

fn main() {
    crash::install();
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

    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            fonts::register(cx);
            ui::theme::set_typography(settings.typography());
            ui::molecules::text_input::bind_keys(cx);
            ui::molecules::command_palette::bind_keys(cx);
            features::keyboard::bind_keys(cx);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new(&crate::platform::key("cmd-q"), Quit, None)]);
            cx.on_window_closed(|cx| cx.quit()).detach();
            features::menus::set_menus(cx, &[], settings.theme);

            let bounds = Bounds::centered(
                None,
                dev::window_size().unwrap_or(size(px(1280.), px(820.))),
                cx,
            );
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
            cx.open_window(opts, |window, cx| {
                cx.new(|cx| app::GitViewer::new(engine.clone(), settings.clone(), window, cx))
            })
            .expect("open window");
            cx.activate(true);
        });
}
