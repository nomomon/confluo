//! The list's involvement tabs as a strip of pages (`state::list_tabs`): at rest only the current tab's
//! rows; while a swipe or a tab change moves the strip, the neighbor tabs draw beside it (each with its
//! own scroll position; pages tile exactly and the strip clips, so they never overlap), and pages away
//! from the center dim slightly. Tried: a 30% parallax on the incoming page read as lag; a plain 1:1
//! slide with a light dim felt most native.

use gpui::{div, prelude::*, px, AnyElement, Context, Hsla};

use crate::app::GitViewer;
use crate::features::shared::layout;
use crate::state::{list_tabs_index, INVOLVEMENTS};
use crate::ui::motion::interactive::page_offsets;
use crate::ui::theme;

/// Dimming of a page one full width away from the center.
const DIM: f32 = 0.08;

impl GitViewer {
    pub(super) fn render_list_pages(
        &self,
        current: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = &self.review.swipe.tabs;
        let cur = list_tabs_index(self.tab);
        let moving = t.pager.moving() || crate::dev::list_tabs().is_some();
        let pages = crate::state::list_tabs_pages(cur, t.progress, moving);
        let wheel = cx.listener(Self::list_tabs_wheel);
        // Measure the list area every frame: the strip steps by exactly this width.
        let cell = t.measured.clone();
        let measure = gpui::canvas(
            move |b, _, _| cell.set(f32::from(b.size.width)),
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        if pages.len() == 1 && !moving {
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h(theme::space_0())
                .relative()
                .on_scroll_wheel(wheel)
                .child(measure)
                .child(current)
                .into_any_element();
        }
        let offsets = page_offsets(t.x, t.width, INVOLVEMENTS.len());
        let mut strip = div()
            .relative()
            .flex_1()
            .min_h(theme::space_0())
            .overflow_hidden()
            .on_scroll_wheel(wheel)
            .child(measure);
        let mut current = Some(current);
        for i in pages {
            let body: AnyElement = if i == cur {
                match current.take() {
                    Some(c) => c,
                    None => continue,
                }
            } else {
                let items = t.cache[i]
                    .clone()
                    .unwrap_or_else(|| self.listing_for(INVOLVEMENTS[i].1));
                let refs: Vec<&gv_engine::ChangeRequest> = items.iter().collect();
                let key = ["pr-list-tab0", "pr-list-tab1", "pr-list-tab2"][i];
                let h = t.scrolls[i].clone();
                let col = crate::ui::elastic::content(key)
                    .flex()
                    .flex_col()
                    .children(self.render_sections(&refs, cx));
                let pane = div()
                    .id(key)
                    .track_scroll(&h)
                    .size_full()
                    .map(crate::ui::scroll::y_only)
                    .pb(layout::END_PAD)
                    .child(col);
                crate::ui::elastic::y_scroller(pane, key, h).into_any_element()
            };
            let away = (t.progress - i as f32).abs().min(1.);
            strip = strip.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .w(px(t.width))
                    .left(px(offsets[i]))
                    .flex()
                    .flex_col()
                    .child(body)
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .bg(Hsla::black().opacity(DIM * away)),
                    ),
            );
        }
        strip.into_any_element()
    }
}
