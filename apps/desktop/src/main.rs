//! git-viewer desktop app (GPUI).

mod app;
mod assets;
mod features;
mod settings;
mod state;
mod ui;

use gpui::{actions, point, px, size, App, AppContext, Application, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions};
use std::sync::Arc;

actions!(git_viewer, [Quit]);

fn main() {
    let engine = match gv_engine::Engine::open_default() {
        Ok(e) => Arc::new(e),
        Err(e) => {
            eprintln!("git-viewer: cannot open cache: {e:#}");
            std::process::exit(1);
        }
    };
    let settings = settings::Settings::load();
    ui::motion::init();

    Application::new().with_assets(assets::Assets).run(move |cx: &mut App| {
        ui::molecules::text_input::bind_keys(cx);
        features::keyboard::bind_keys(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.on_window_closed(|cx| cx.quit()).detach();

        let bounds = Bounds::centered(None, size(px(1280.), px(820.)), cx);
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("git-viewer".into()),
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
