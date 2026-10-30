//! Dev-only launch hooks read from the environment, for screenshots and walkthroughs.
//! Documented in `docs/development.md`. None of them is needed in normal use.

use gpui::{px, size, Pixels, Size};

use crate::settings::{Appearance, Settings, ThemeMode};

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// `CONFLUO_WINDOW=1100x760` opens the window at that size.
pub fn window_size() -> Option<Size<Pixels>> {
    let v = var("CONFLUO_WINDOW")?;
    let (w, h) = v.split_once('x')?;
    Some(size(px(w.trim().parse().ok()?), px(h.trim().parse().ok()?)))
}

/// `CONFLUO_THEME=light|dark|system` and `CONFLUO_MATERIAL=blurred|transparent|opaque`
/// (and `CONFLUO_SPLIT=1`, side-by-side diff) override the saved appearance for this launch (saved again only if Settings change).
pub fn apply_overrides(s: &mut Settings) {
    match var("CONFLUO_THEME").as_deref() {
        Some("light") => s.theme = ThemeMode::Light,
        Some("dark") => s.theme = ThemeMode::Dark,
        Some("system") => s.theme = ThemeMode::System,
        _ => {}
    }
    if var("CONFLUO_SPLIT").is_some() {
        s.diff_split = true;
    }
    match var("CONFLUO_MATERIAL").as_deref() {
        Some("blurred") => s.appearance = Appearance::Blurred,
        Some("transparent") => s.appearance = Appearance::Transparent,
        Some("opaque") => s.appearance = Appearance::Opaque,
        _ => {}
    }
}

/// `CONFLUO_NO_REFRESH=1` keeps the app on cached data: no refresh at launch, on focus or on the timer.
pub fn no_refresh() -> bool {
    var("CONFLUO_NO_REFRESH").is_some()
}

/// `CONFLUO_HINTS=1` keeps the hold-Cmd shortcut hints visible (screenshots).
pub fn hints() -> bool {
    var("CONFLUO_HINTS").is_some()
}

/// `CONFLUO_VIEW=starred|bots|project:<name>` opens that list view at launch.
pub fn view() -> Option<String> {
    var("CONFLUO_VIEW")
}

/// `CONFLUO_SCROLL=<px>` scrolls the detail pane down that far once it has laid out.
pub fn detail_scroll() -> Option<f32> {
    var("CONFLUO_SCROLL")?.parse().ok()
}

/// `CONFLUO_OVERLAY=add-project|rename|delete|menu` opens that sheet, dialog or menu at launch.
pub fn overlay() -> Option<String> {
    var("CONFLUO_OVERLAY")
}

/// `CONFLUO_DEV_MARKDOWN=<file>` replaces the open PR's description and every comment body with that markdown file
/// (wide tables and code blocks in their real containers). Read once.
pub fn markdown_override() -> Option<&'static str> {
    static SRC: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    SRC.get_or_init(|| var("CONFLUO_DEV_MARKDOWN").and_then(|p| std::fs::read_to_string(p).ok())).as_deref()
}
