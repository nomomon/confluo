//! Appearance dialog: window background, opacity, image, dim and tint.

use gpui::{div, prelude::*, px, ClickEvent, Context, PathPromptOptions};

use crate::app::GitViewer;
use crate::settings::Appearance;
use crate::ui::atoms::{Button, Swatch};
use crate::ui::molecules::{Field, SegmentedTabs};
use crate::ui::theme;

const OPS: [f32; 6] = [0.25, 0.4, 0.55, 0.7, 0.85, 1.0];
const DIMS: [f32; 5] = [0.0, 0.25, 0.45, 0.65, 0.8];
const TINTS: [&str; 6] = ["#14141c", "#0b1220", "#1a1025", "#0f1a14", "#201410", "#000000"];

fn percent_tabs(id: &str, values: &[f32], current: f32) -> SegmentedTabs {
    values.iter().fold(SegmentedTabs::new(id.to_string()), |t, v| t.tab(format!("{}%", (v * 100.) as u32), (v - current).abs() < 0.01))
}

impl GitViewer {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let s = &self.settings;
        let appearance = Appearance::ALL
            .iter()
            .fold(SegmentedTabs::new("appearance"), |t, a| t.tab(a.label(), *a == s.appearance))
            .on_select(cx.listener(|this, i: &usize, _, cx| {
                this.settings.appearance = Appearance::ALL[*i];
                this.save_settings(cx);
            }));
        let opacity = percent_tabs("opacity", &OPS, s.panel_opacity).on_select(cx.listener(|this, i: &usize, _, cx| {
            this.settings.panel_opacity = OPS[*i];
            this.save_settings(cx);
        }));
        let dim = percent_tabs("dim", &DIMS, s.background_dim).on_select(cx.listener(|this, i: &usize, _, cx| {
            this.settings.background_dim = DIMS[*i];
            this.save_settings(cx);
        }));
        let swatches = div().flex().gap_2().children(TINTS.iter().map(|t| {
            let t = *t;
            Swatch::new(format!("tint-{t}"), theme::parse_hex(t).unwrap_or(theme::panel(1.)))
                .active(s.tint.eq_ignore_ascii_case(t))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.settings.tint = t.to_string();
                    this.tint_input.update(cx, |i, cx| i.set_text(t, cx));
                    this.save_settings(cx);
                }))
        }));
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(Field::new("Window background", appearance))
            .child(Field::new("Panel opacity", opacity))
            .child(Field::new("Background image", self.render_bg_image_row(cx)))
            .child(Field::new("Image dim", dim))
            .child(Field::new("Tint", div().flex().gap_3().items_center().child(swatches).child(div().w(px(110.)).child(self.tint_input.clone()))))
            .child(div().text_xs().text_color(theme::faint()).child(format!("Saved to {}", crate::settings::path().display())))
    }

    fn render_bg_image_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap_2()
            .items_center()
            .child(div().flex_1().child(self.bg_input.clone()))
            .child(Button::new("bg-browse", "Choose...").on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                let rx = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Choose".into()) });
                cx.spawn(async move |this, cx| {
                    if let Ok(Ok(Some(paths))) = rx.await {
                        if let Some(p) = paths.into_iter().next() {
                            let p = p.to_string_lossy().to_string();
                            this.update(cx, |v, cx| {
                                v.bg_input.update(cx, |i, cx| i.set_text(p.clone(), cx));
                                v.settings.background_image = Some(p);
                                v.save_settings(cx);
                            })
                            .ok();
                        }
                    }
                })
                .detach();
            })))
            .child(Button::new("bg-clear", "Clear").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.settings.background_image = None;
                this.bg_input.update(cx, |i, cx| i.set_text("", cx));
                this.save_settings(cx);
            })))
    }
}
