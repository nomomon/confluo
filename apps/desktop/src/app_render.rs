//! Root view rendering: window composition per mode, triage layout and surfaces.

use crate::app::GitViewer;
use crate::features::layout;
use crate::ui::molecules::{Toast, ToastVariant, Toaster};
use crate::ui::theme::{self, Translucency};
use gpui::{div, prelude::*, Context, Pixels, Window};

impl GitViewer {
    /// Sidebar, list and (when open) the preview: the window outside Review mode, also drawn under a
    /// review that slides in or out. Panels open and close through `panel_frame` (width spring).
    pub(crate) fn render_triage(&self, cx: &mut Context<Self>) -> gpui::Div {
        let sidebar = self.panel_frame(
            crate::state::Pane::Sidebar,
            div().h_full().child(self.render_sidebar(cx)),
            cx,
        );
        let preview = self.detail.as_ref().and_then(|d| {
            self.panel_frame(crate::state::Pane::Preview, self.render_preview(d, cx), cx)
        });
        div()
            .relative()
            .flex()
            .size_full()
            .children(sidebar)
            .child(self.render_list(cx))
            .children(preview)
            .children(self.panel_edge(crate::state::Pane::Sidebar, cx))
            .children(
                self.detail
                    .as_ref()
                    .and_then(|_| self.panel_edge(crate::state::Pane::Preview, cx)),
            )
    }

    /// Sidebar width when shown (rail in compact layouts).
    pub(crate) fn sidebar_w_open(&self) -> Pixels {
        if self.compact {
            layout::sidebar_rail_w()
        } else {
            layout::sidebar_w()
        }
    }

    /// Panel, card and window fills derived from the appearance settings.
    pub(crate) fn surfaces(&self) -> Translucency {
        self.settings.surfaces()
    }
}

impl Render for GitViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.fetch_avatars(cx);
        crate::ui::elastic::begin_frame(window);
        self.swipe_frame(window, cx);
        self.split_frame(window, cx);
        // Notes drawn this frame queue their award emoji; fetch after the frame is built.
        cx.defer_in(window, |this, _, cx| this.flush_awards(cx));
        if std::mem::take(&mut self.refocus) {
            window.focus(&self.focus);
        }
        if std::mem::take(&mut self.refocus_alias) {
            window.focus(&gpui::Focusable::focus_handle(
                self.alias_input.read(cx),
                cx,
            ));
        }
        if std::mem::take(&mut self.refocus_quick) {
            window.focus(&gpui::Focusable::focus_handle(
                self.quick_input.read(cx),
                cx,
            ));
        }
        if std::mem::take(&mut self.refocus_palette) {
            window.focus(&gpui::Focusable::focus_handle(
                self.palette_input.read(cx),
                cx,
            ));
        }
        if let Some(a) = self.pending_action.take() {
            window.focus(&self.focus);
            window.dispatch_action(a, cx);
        }
        let names: Vec<String> = self.projects.iter().map(|p| p.name.clone()).collect();
        let menu_key = (names, self.settings.theme, self.cmd_ctx().draft);
        if self.menu_projects.as_ref() != Some(&menu_key) {
            crate::features::menus::set_menus(cx, &menu_key.0, menu_key.1, menu_key.2);
            self.menu_projects = Some(menu_key);
        }
        theme::set_typography(self.settings.typography());
        theme::set_light(self.settings.theme.is_light(window.appearance()));
        window.set_background_appearance(self.settings.appearance.to_gpui());
        self.viewport_w = window.viewport_size().width;
        // The triage layout as it is (or will be, under Review mode: the swipe underlay is the exact list
        // view that comes back, with the preview open on the same PR).
        let previewing = !self.preview_hidden && self.detail.is_some();
        self.compact = previewing && self.viewport_w < layout::PREVIEW_COMPACT_BELOW;
        // One floating layer at a time: hold-Cmd hints stay hidden while a menu or popover is open.
        self.hint_ctx =
            (self.hints.visible() && self.settings.shortcut_hints && !self.layer_open())
                .then(|| self.cmd_ctx());
        crate::ui::layer::begin_frame();
        let main = div().absolute().top_0().left_0().size_full().flex();
        let main = match self.detail.as_ref().filter(|_| self.reviewing) {
            // Review mode: the PR takes the whole content area; the sidebar and list step aside.
            Some(d) => {
                let review = self.render_review(d, cx).into_any_element();
                let x = self.review.swipe.page_x;
                let review = if x != 0. {
                    self.render_swipe_stack(review, x, cx)
                } else {
                    review
                };
                // Reduced motion: the view a swipe switched to fades in.
                main.child(match self.review.swipe.fade {
                    Some(n) => crate::ui::motion::fade_in_always(
                        div().size_full().child(review),
                        ("swipe-fade", n),
                    ),
                    None => review,
                })
            }
            None => main.child(self.render_triage(cx)),
        };
        let main = match self.review.swipe.fade.filter(|_| !self.reviewing) {
            Some(n) => div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(crate::ui::motion::fade_in_always(main, ("swipe-fade", n)))
                .into_any_element(),
            None => main.into_any_element(),
        };
        let root = div().id("root");
        self.key_handlers(root, cx)
            .on_modifiers_changed(cx.listener(Self::hint_modifiers))
            .on_scroll_wheel(cx.listener(Self::page_wheel))
            .relative()
            .size_full()
            .font_family(theme::ui_font())
            .text_color(theme::text())
            .text_size(theme::text_sm())
            .line_height(gpui::relative(theme::LINE_HEIGHT))
            .bg(self.surfaces().window_bg())
            .when(self.review.split.drag.is_some(), |root| {
                self.panel_drag_handlers(root, cx)
            })
            .child(main)
            .children(self.toast.clone().map(|t| {
                let toast = match &self.toast_error {
                    Some(why) => Toast::new(t)
                        .description(why.clone())
                        .variant(ToastVariant::Error),
                    None if self.toast_plain => Toast::new(t),
                    None => Toast::new(t).variant(ToastVariant::Success),
                };
                let toast = if self.quiet.undo.is_some() {
                    toast.action(
                        "Undo",
                        cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.undo_snooze(cx)),
                    )
                } else if let Some(file) = self.toast_file.clone() {
                    toast.action(
                        "Open log",
                        move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut gpui::App| {
                            cx.open_with_system(&file)
                        },
                    )
                } else {
                    toast
                };
                Toaster::new().toast(toast)
            }))
            .children(self.render_overlay(cx))
            .children(self.render_repo_menu(cx))
    }
}
