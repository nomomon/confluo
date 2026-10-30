//! Searchable multi-select popover like shadcn `Combobox`: a search field over a list of
//! options with check marks. The caller owns the query input and filtering (often remote).
//! Keyboard: Up/Down move the highlight (`on_highlight`), Enter in the input toggles it
//! (the caller listens to the input's Submit), Esc dismisses (root handler).
//!
//! `Combobox::new("reviewers", input).option(ComboOption::new("ada").selected(true)).on_toggle(..)`

use gpui::{
    anchored, deferred, div, prelude::*, px, AnyElement, App, ClickEvent, ElementId, Entity, Image,
    KeyDownEvent, MouseDownEvent, Pixels, SharedString, Window,
};
use std::rc::Rc;
use std::sync::Arc;

use super::text_input::TextInput;
use crate::ui::atoms::{Avatar, Icon, IconName, ListItem, Spinner};
use crate::ui::{theme, ChangeHandler};

const PANEL_W: Pixels = px(280.);
const LIST_MAX_H: Pixels = px(260.);

pub struct ComboOption {
    label: SharedString,
    detail: Option<SharedString>,
    avatar: Option<Option<Arc<Image>>>,
    selected: bool,
}

impl ComboOption {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            detail: None,
            avatar: None,
            selected: false,
        }
    }

    /// Muted text after the label (full name, email).
    pub fn detail(mut self, d: Option<impl Into<SharedString>>) -> Self {
        self.detail = d.map(Into::into);
        self
    }

    /// Show an avatar (initial while the image loads).
    pub fn avatar(mut self, img: Option<Arc<Image>>) -> Self {
        self.avatar = Some(img);
        self
    }

    pub fn selected(mut self, on: bool) -> Self {
        self.selected = on;
        self
    }
}

#[derive(IntoElement)]
pub struct Combobox {
    id: SharedString,
    input: Entity<TextInput>,
    options: Vec<ComboOption>,
    highlight: usize,
    loading: bool,
    empty: SharedString,
    on_toggle: Option<ChangeHandler<usize>>,
    on_highlight: Option<ChangeHandler<usize>>,
    on_dismiss: Option<ChangeHandler<MouseDownEvent>>,
}

impl Combobox {
    pub fn new(id: impl Into<SharedString>, input: Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            input,
            options: vec![],
            highlight: 0,
            loading: false,
            empty: "No matches".into(),
            on_toggle: None,
            on_highlight: None,
            on_dismiss: None,
        }
    }

    pub fn option(mut self, o: ComboOption) -> Self {
        self.options.push(o);
        self
    }

    pub fn highlight(mut self, i: usize) -> Self {
        self.highlight = i;
        self
    }

    pub fn loading(mut self, on: bool) -> Self {
        self.loading = on;
        self
    }

    pub fn empty_text(mut self, t: impl Into<SharedString>) -> Self {
        self.empty = t.into();
        self
    }

    pub fn on_toggle(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
        self
    }

    pub fn on_highlight(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_highlight = Some(Rc::new(f));
        self
    }

    pub fn on_dismiss(
        mut self,
        f: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }

    fn row(&self, i: usize, o: ComboOption) -> AnyElement {
        let f = self.on_toggle.clone();
        div()
            .id(ElementId::Name(format!("{}-opt-{i}", self.id).into()))
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .px(theme::SPACE_2)
            .py(theme::item_pad_y())
            .list_item_radius(i == self.highlight, theme::RADIUS_SM)
            .children(o.avatar.map(|img| Avatar::new(o.label.clone()).image(img)))
            .child(div().flex_shrink_0().child(o.label))
            .child(
                div()
                    .flex_1()
                    .min_w(theme::SPACE_0)
                    .truncate()
                    .text_color(theme::text_subtle())
                    .children(o.detail),
            )
            .when(o.selected, |d| {
                d.child(
                    Icon::new(IconName::Check)
                        .size(theme::ICON_SM)
                        .color(theme::text()),
                )
            })
            .on_click(move |_: &ClickEvent, window, cx| {
                if let Some(f) = &f {
                    f(&i, window, cx)
                }
            })
            .into_any_element()
    }
}

impl RenderOnce for Combobox {
    fn render(mut self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let n = self.options.len();
        let (hl, on_hl) = (self.highlight, self.on_highlight.clone());
        let options = std::mem::take(&mut self.options);
        let rows: Vec<AnyElement> = options
            .into_iter()
            .enumerate()
            .map(|(i, o)| self.row(i, o))
            .collect();
        let status = if self.loading {
            Some(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::SPACE_2)
                    .child(Spinner::new(format!("{}-spin", self.id)).size(theme::ICON_SM))
                    .child("Searching...")
                    .into_any_element(),
            )
        } else if n == 0 {
            Some(div().child(self.empty.clone()).into_any_element())
        } else {
            None
        };
        let dismiss = self.on_dismiss.clone();
        let search = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_1)
            .pl(theme::SPACE_2)
            .h(theme::control_h())
            .border_b_1()
            .border_color(theme::border_subtle())
            .child(
                Icon::new(IconName::Search)
                    .size(theme::ICON_SM)
                    .color(theme::text_subtle()),
            )
            .child(div().flex_1().child(self.input.clone()));
        let panel = crate::ui::layer::track(div())
            .id(ElementId::Name(format!("{}-panel", self.id).into()))
            .mt(theme::SPACE_1)
            .w(PANEL_W)
            .flex()
            .flex_col()
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(theme::border())
            .bg(theme::overlay())
            .shadow_lg()
            .text_size(theme::text_sm())
            .on_mouse_down_out(move |ev: &MouseDownEvent, w, cx| {
                if let Some(f) = &dismiss {
                    f(ev, w, cx)
                }
            })
            .on_key_down(move |ev: &KeyDownEvent, window, cx| {
                let next = match ev.keystroke.key.as_str() {
                    "down" => (hl + 1).min(n.saturating_sub(1)),
                    "up" => hl.saturating_sub(1),
                    _ => return,
                };
                if let Some(f) = &on_hl {
                    f(&next, window, cx)
                }
                cx.stop_propagation();
            })
            .child(search)
            .child(
                div()
                    .id(ElementId::Name(format!("{}-list", self.id).into()))
                    .max_h(LIST_MAX_H)
                    .map(crate::ui::scroll::y_only)
                    .p(theme::SPACE_1)
                    .flex()
                    .flex_col()
                    .children(rows)
                    .children(status.map(|s| {
                        div()
                            .px(theme::SPACE_2)
                            .py(theme::item_pad_y())
                            .text_color(theme::text_subtle())
                            .child(s)
                    })),
            );
        deferred(
            anchored()
                .snap_to_window_with_margin(theme::SPACE_2)
                .child(panel),
        )
        .with_priority(1)
    }
}
