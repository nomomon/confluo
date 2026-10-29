//! Rounded card holding list rows, with an optional collapsible header and count.
//!
//! `ListSection::new(id, fill).header("Needs you", 3).open(true).on_toggle(..).child(row)`

use gpui::{div, prelude::*, AnyElement, App, ClickEvent, ElementId, Hsla, SharedString, Window};

use crate::ui::atoms::{Icon, IconName};
use crate::ui::{theme, ClickHandler};

#[derive(IntoElement)]
pub struct ListSection {
    id: SharedString,
    fill: Hsla,
    header: Option<(SharedString, usize)>,
    open: bool,
    on_toggle: Option<ClickHandler>,
    children: Vec<AnyElement>,
}

impl ListSection {
    pub fn new(id: impl Into<SharedString>, fill: Hsla) -> Self {
        Self { id: id.into(), fill, header: None, open: true, on_toggle: None, children: vec![] }
    }

    pub fn header(mut self, title: impl Into<SharedString>, count: usize) -> Self {
        self.header = Some((title.into(), count));
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn on_toggle(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }
}

impl ParentElement for ListSection {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for ListSection {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let open = self.open || self.header.is_none();
        let header = self.header.map(|(title, count)| {
            div()
                .id(ElementId::Name(format!("{}-header", self.id).into()))
                .flex()
                .items_center()
                .gap(theme::SPACE_2)
                .px(theme::SPACE_3)
                .py(theme::SPACE_2)
                .cursor_pointer()
                .text_size(theme::text_xs())
                .text_color(theme::text_muted())
                .hover(|s| s.text_color(theme::text()))
                .child(Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight }).size(theme::ICON_SM).color(theme::text_subtle()))
                .child(div().font_weight(theme::WEIGHT_MEDIUM).child(title))
                .child(div().text_color(theme::text_subtle()).child(count.to_string()))
                .when_some(self.on_toggle, |el, f| el.on_click(f))
        });
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .p(theme::SPACE_1)
            .rounded(theme::RADIUS_LG)
            .border_1()
            .border_color(theme::border_subtle())
            .bg(self.fill)
            .children(header)
            .when(open, |d| d.children(self.children))
    }
}
