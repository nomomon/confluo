//! Handlers for the view commands: theme, zoom, full screen, word wrap, find, go to line, recent
//! PRs and the developer commands. Bindings and labels live in `keymap`.

use gpui::{prelude::*, Context};

use super::keymap::*;
use crate::app::GitViewer;
use crate::settings::ThemeMode;

impl GitViewer {
    pub(crate) fn view_handlers<E: InteractiveElement>(&self, el: E, cx: &mut Context<Self>) -> E {
        el.on_action(cx.listener(|this, _: &ToggleTheme, _, cx| this.cycle_theme(cx)))
            .on_action(
                cx.listener(|this, _: &ThemeLight, _, cx| this.set_theme(ThemeMode::Light, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ThemeDark, _, cx| this.set_theme(ThemeMode::Dark, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ThemeSystem, _, cx| this.set_theme(ThemeMode::System, cx)),
            )
            .on_action(cx.listener(|this, _: &ZoomIn, _, cx| this.zoom(Some(1), cx)))
            .on_action(cx.listener(|this, _: &ZoomOut, _, cx| this.zoom(Some(-1), cx)))
            .on_action(cx.listener(|this, _: &ZoomReset, _, cx| this.zoom(None, cx)))
            .on_action(cx.listener(|_, _: &ToggleFullScreen, window, _| window.toggle_fullscreen()))
            .on_action(cx.listener(|this, _: &ToggleWrap, _, cx| {
                if this.reviewing {
                    this.toggle_wrap(cx)
                }
            }))
            .on_action(cx.listener(|this, _: &GoToLine, window, cx| {
                if this.reviewing {
                    this.open_goto_line(window, cx)
                }
            }))
            .on_action(cx.listener(|this, _: &FindPrev, _, cx| {
                if this.find.open {
                    this.find_step(-1, cx)
                }
            }))
            .on_action(cx.listener(|this, _: &OpenRecent, window, cx| {
                this.open_palette_with(crate::state::palette_rank::RECENT_PREFIX, window, cx)
            }))
            .on_action(cx.listener(|this, _: &OpenLogs, _, cx| this.open_logs(cx)))
            .on_action(cx.listener(|this, _: &DevReload, window, cx| this.dev_reload(window, cx)))
    }
}
