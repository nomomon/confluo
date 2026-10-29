//! Whole-file viewer: mono lines with a line number gutter, virtualized.

use gpui::{div, prelude::*, uniform_list, AnyElement, ClickEvent, Context};

use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::FileView;
use crate::ui::atoms::{Button, ButtonVariant, ErrorText, IconName};
use crate::ui::molecules::code::{DiffRow, Lang, LineMark};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_file_viewer(&self, v: &FileView, cx: &mut Context<Self>) -> AnyElement {
        let bar = div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .child(Button::new("viewer-back", "Back to diff").icon(IconName::ArrowLeft).variant(ButtonVariant::Ghost).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    if let Some(d) = this.detail.as_mut() {
                        d.code.viewer = None;
                    }
                    cx.notify()
                },
            )))
            .child(div().flex_1().min_w(theme::SPACE_0).truncate().font_family(theme::mono_font()).text_size(theme::text_xs()).child(v.path.clone()));
        let body: AnyElement = match (&v.lines, &v.error) {
            (_, Some(e)) => ErrorText::new(e.clone()).into_any_element(),
            (None, None) => div().text_size(theme::text_sm()).text_color(theme::text_muted()).child("Loading file...").into_any_element(),
            (Some(lines), None) => {
                let lines = lines.clone();
                let lang = Lang::from_path(&v.path);
                let digits = lines.len().to_string().len() as f32;
                let gutter = (theme::text_xs() * layout::MONO_CHAR_RATIO * digits + theme::SPACE_4).max(layout::DIFF_GUTTER_W);
                uniform_list("file-viewer", lines.len(), move |range, _, _| {
                    range
                        .map(|i| {
                            DiffRow::new(LineMark::Context, lines[i].clone(), lang)
                                .numbers(None, Some(i as u32 + 1))
                                .single_gutter()
                                .gutter_w(gutter)
                                .height(layout::DIFF_LINE_H)
                        })
                        .collect()
                })
                .track_scroll(v.scroll.clone())
                .flex_1()
                .h_full()
                .py(theme::SPACE_2)
                .rounded(theme::RADIUS_LG)
                .border_1()
                .border_color(theme::border_subtle())
                .bg(self.surfaces().card())
                .into_any_element()
            }
        };
        div().flex().flex_col().flex_1().min_h(theme::SPACE_0).gap(theme::SPACE_3).child(bar).child(body).into_any_element()
    }
}
