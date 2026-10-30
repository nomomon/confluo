//! View preferences changed by commands: theme, zoom, diff word wrap, and developer commands.

use gpui::{Context, Window};

use crate::app::GitViewer;
use crate::settings::{zoom_step, ThemeMode};

impl GitViewer {
    pub(crate) fn set_theme(&mut self, mode: ThemeMode, cx: &mut Context<Self>) {
        self.settings.theme = mode;
        self.save_settings(cx);
        self.show_info_toast(format!("Theme: {}", mode.label()), cx);
    }

    /// Cmd+Shift+L: Light, Dark, System, Light.
    pub(crate) fn cycle_theme(&mut self, cx: &mut Context<Self>) {
        self.set_theme(self.settings.theme.next(), cx)
    }

    /// Cmd+= / Cmd+- step the zoom (`delta`), Cmd+0 resets it (`None`).
    pub(crate) fn zoom(&mut self, delta: Option<isize>, cx: &mut Context<Self>) {
        let z = match delta {
            Some(d) => zoom_step(self.settings.zoom, d),
            None => 1.0,
        };
        self.settings.zoom = z;
        crate::ui::theme::set_typography(self.settings.typography());
        self.remeasure_diff();
        self.save_settings(cx);
        self.show_info_toast(format!("Zoom {}%", (z * 100.).round()), cx);
    }

    /// Alt+Z: wrap long diff lines (rows grow) or scroll them sideways.
    pub(crate) fn toggle_wrap(&mut self, cx: &mut Context<Self>) {
        self.settings.diff_wrap = !self.settings.diff_wrap;
        self.remeasure_diff();
        self.save_settings(cx);
        let state = if self.settings.diff_wrap { "on" } else { "off" };
        self.show_info_toast(format!("Word wrap {state}"), cx);
    }

    /// Row heights changed (wrap, zoom): measure every row again, keeping the scroll position.
    fn remeasure_diff(&mut self) {
        if let Some(d) = self.detail.as_mut() {
            let top = d.code.list.logical_scroll_top();
            d.code.rebuild();
            d.code.list.scroll_to(top);
        }
    }

    /// Developer: Open logs folder (crash reports).
    pub(crate) fn open_logs(&mut self, cx: &mut Context<Self>) {
        let dir = crate::crash::log_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.show_error_toast("Could not open the logs folder", e.to_string(), cx);
            return;
        }
        cx.open_with_system(&dir);
    }

    /// Developer: Reload: re-read lists and diffs from the local cache and repaint, no network.
    pub(crate) fn dev_reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reload();
        self.remeasure_diff();
        self.find.matches.clear();
        if self.find.open {
            self.run_find(cx);
        }
        window.refresh();
        self.show_info_toast("Reloaded from the local cache", cx);
    }
}
