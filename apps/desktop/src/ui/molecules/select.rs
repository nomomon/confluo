//! Single-choice dropdown like shadcn `Select`: a trigger showing the value and a popover list.
//! Keyboard: focus the trigger, Up/Down open and move, Enter or Space picks, Esc closes.
//!
//! `Select::new("ui-font").option("Inter").font_option("Geist", "Geist").selected(1).on_change(..)`

use gpui::{
    anchored, deferred, div, prelude::*, px, App, ClickEvent, ElementId, Entity, FocusHandle,
    KeyDownEvent, MouseDownEvent, SharedString, Window,
};
use std::rc::Rc;

use crate::ui::atoms::{Icon, IconName, ListItem};
use crate::ui::{theme, ChangeHandler};

struct SelectState {
    open: bool,
    highlight: usize,
    focus: FocusHandle,
}

struct Opt {
    label: SharedString,
    font: Option<SharedString>,
}

#[derive(IntoElement)]
pub struct Select {
    id: SharedString,
    options: Vec<Opt>,
    selected: usize,
    on_change: Option<ChangeHandler<usize>>,
}

impl Select {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            options: vec![],
            selected: 0,
            on_change: None,
        }
    }

    pub fn option(mut self, label: impl Into<SharedString>) -> Self {
        self.options.push(Opt {
            label: label.into(),
            font: None,
        });
        self
    }

    /// An option rendered in its own font family (font pickers).
    pub fn font_option(
        mut self,
        label: impl Into<SharedString>,
        family: impl Into<SharedString>,
    ) -> Self {
        self.options.push(Opt {
            label: label.into(),
            font: Some(family.into()),
        });
        self
    }

    pub fn selected(mut self, i: usize) -> Self {
        self.selected = i;
        self
    }

    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

fn set(state: &Entity<SelectState>, cx: &mut App, f: impl FnOnce(&mut SelectState)) {
    state.update(cx, |s, cx| {
        f(s);
        cx.notify()
    });
}

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let selected = self.selected;
        let state = window.use_keyed_state(
            ElementId::Name(format!("{}-state", self.id).into()),
            cx,
            |_, cx| SelectState {
                open: false,
                highlight: selected,
                focus: cx.focus_handle(),
            },
        );
        let (open, hl, focus) = {
            let s = state.read(cx);
            (s.open, s.highlight, s.focus.clone())
        };
        let n = self.options.len();
        let current = self
            .options
            .get(selected)
            .map(|o| (o.label.clone(), o.font.clone()));
        let on_change = self.on_change.clone();
        let (st_click, st_key, st_focus) = (state.clone(), state.clone(), focus.clone());
        let trigger = div()
            .id(ElementId::Name(self.id.clone()))
            .track_focus(&focus)
            .flex()
            .items_center()
            .gap(theme::space_2())
            .min_w(px(160.))
            .h(px(30.))
            .px(theme::space_2p5())
            .rounded(theme::RADIUS_MD)
            .border_1()
            .border_color(if focus.is_focused(window) {
                theme::focus_ring()
            } else {
                theme::border()
            })
            .bg(theme::surface())
            .text_size(theme::text_sm())
            .cursor_pointer()
            .hover(|s| s.bg(theme::surface_hover()))
            .child(
                div()
                    .flex_1()
                    .truncate()
                    .when_some(current.as_ref().and_then(|c| c.1.clone()), |d, f| {
                        d.font_family(f)
                    })
                    .children(current.map(|c| c.0)),
            )
            .child(Icon::new(IconName::Selector).size(theme::icon_sm()))
            .on_click(move |_: &ClickEvent, window, cx| {
                window.focus(&st_focus);
                set(&st_click, cx, |s| {
                    s.open = !s.open;
                    s.highlight = selected;
                });
            })
            .on_key_down(move |ev: &KeyDownEvent, window, cx| {
                let pick = |i: usize, window: &mut Window, cx: &mut App| {
                    if let Some(f) = &on_change {
                        f(&i, window, cx)
                    }
                };
                match ev.keystroke.key.as_str() {
                    "down" | "up" if !open => set(&st_key, cx, |s| s.open = true),
                    "down" => set(&st_key, cx, |s| {
                        s.highlight = (s.highlight + 1).min(n.saturating_sub(1))
                    }),
                    "up" => set(&st_key, cx, |s| s.highlight = s.highlight.saturating_sub(1)),
                    "enter" | "space" if open => {
                        set(&st_key, cx, |s| s.open = false);
                        pick(hl, window, cx);
                    }
                    "enter" | "space" => set(&st_key, cx, |s| s.open = true),
                    "escape" if open => set(&st_key, cx, |s| s.open = false),
                    _ => return,
                }
                cx.stop_propagation();
            });
        let list = open.then(|| {
            let st_out = state.clone();
            let panel = crate::ui::layer::track(div())
                .mt(theme::space_1())
                .min_w(px(160.))
                .p(theme::space_1())
                .flex()
                .flex_col()
                .rounded(theme::RADIUS_LG)
                .border_1()
                .border_color(theme::border())
                .bg(theme::overlay())
                .shadow_lg()
                .text_size(theme::text_sm())
                .on_mouse_down_out(move |_: &MouseDownEvent, _, cx| {
                    set(&st_out, cx, |s| s.open = false)
                })
                .children(self.options.into_iter().enumerate().map(|(i, o)| {
                    let (st, f) = (state.clone(), self.on_change.clone());
                    div()
                        .id(ElementId::Name(format!("{}-opt-{i}", self.id).into()))
                        .flex()
                        .items_center()
                        .gap(theme::space_2())
                        .px(theme::space_2())
                        .py(theme::item_pad_y())
                        .list_item_radius(i == hl, theme::RADIUS_SM)
                        .child(
                            div()
                                .flex_1()
                                .when_some(o.font, |d, f| d.font_family(f))
                                .child(o.label),
                        )
                        .when(i == selected, |d| {
                            d.child(
                                Icon::new(IconName::Check)
                                    .size(theme::icon_sm())
                                    .color(theme::text()),
                            )
                        })
                        .on_click(move |_: &ClickEvent, window, cx| {
                            set(&st, cx, |s| s.open = false);
                            if let Some(f) = &f {
                                f(&i, window, cx)
                            }
                        })
                }));
            deferred(
                anchored()
                    .snap_to_window_with_margin(theme::space_2())
                    .child(panel),
            )
            .with_priority(1)
        });
        div().flex().flex_col().child(trigger).children(list)
    }
}
