//! Mutually exclusive tabs like shadcn `Tabs`: `Pill` (segmented, default), `Underline`, or
//! `Floating` (pill bar on an overlay with a shadow, for bottom tab bars).
//!
//! `Tabs::new("detail").variant(TabsVariant::Underline).tab("Code", true).on_select(..)`

use gpui::{div, prelude::*, px, AnyElement, App, ClickEvent, ElementId, SharedString, Window};
use std::rc::Rc;

use crate::ui::{theme, ChangeHandler};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum TabsVariant {
    #[default]
    Pill,
    Underline,
    #[allow(dead_code)] // kit API
    Floating,
}

#[derive(IntoElement)]
pub struct Tabs {
    id: SharedString,
    items: Vec<(AnyElement, bool)>,
    variant: TabsVariant,
    on_select: Option<ChangeHandler<usize>>,
    progress: Option<f32>,
}

impl Tabs {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self { id: id.into(), items: vec![], variant: TabsVariant::default(), on_select: None, progress: None }
    }

    /// Append a tab; `active` marks the selected one. The label can be any element.
    pub fn tab(mut self, label: impl IntoElement, active: bool) -> Self {
        self.items.push((label.into_any_element(), active));
        self
    }

    /// Underline: fractional selected tab while a pager moves between tabs (0..n-1); the indicator and
    /// label emphasis cross over with it instead of switching.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p);
        self
    }

    pub fn variant(mut self, variant: TabsVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Called with the index of the clicked tab.
    pub fn on_select(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Tabs {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let v = self.variant;
        let row = div().flex().flex_shrink_0().items_center();
        let row = match v {
            TabsVariant::Pill => row.h(theme::control_h()).p(theme::SPACE_0P5).gap(theme::SPACE_0P5).rounded(theme::RADIUS_MD).bg(theme::surface()).border_1().border_color(theme::border()),
            TabsVariant::Floating => row.p(theme::SPACE_1).gap(theme::SPACE_1).rounded(theme::RADIUS_FULL).border_1().border_color(theme::border()).bg(theme::overlay()).shadow_lg(),
            TabsVariant::Underline => row.gap(theme::SPACE_4).border_b_1().border_color(theme::border_subtle()),
        };
        let progress = self.progress;
        row.children(self.items.into_iter().enumerate().map(|(i, (label, active))| {
            // How selected this tab looks: 1 when active, in between while a pager moves.
            let on_k = progress.map_or(if active { 1. } else { 0. }, |p| (1. - (p - i as f32).abs()).clamp(0., 1.));
            let on = self.on_select.clone();
            let tab = div()
                .id(ElementId::Name(format!("{}-{i}", self.id).into()))
                .flex()
                .items_center()
                .gap(theme::SPACE_1P5)
                .text_size(theme::text_sm())
                .font_weight(theme::WEIGHT_MEDIUM)
                .cursor_pointer()
                .text_color(if active { theme::text() } else { theme::text_muted() })
                .hover(|s| s.text_color(theme::text()))
                .child(label)
                .when_some(on, |d, f| d.on_click(move |_: &ClickEvent, w, cx| f(&i, w, cx)));
            match v {
                TabsVariant::Pill => tab.h_full().px(theme::SPACE_3).rounded(theme::RADIUS_SM).when(active, |d| d.bg(theme::surface_active())),
                TabsVariant::Floating => tab.px(theme::SPACE_3).py(theme::SPACE_1).rounded(theme::RADIUS_FULL).when(active, |d| d.bg(theme::surface_active())),
                TabsVariant::Underline => tab.py(theme::SPACE_2).border_b_2().mb(px(-1.)).border_color(theme::text().opacity(on_k)).when(progress.is_some(), |d| d.text_color(if on_k > 0.5 { theme::text() } else { theme::text_muted() })),
            }
        }))
    }
}
