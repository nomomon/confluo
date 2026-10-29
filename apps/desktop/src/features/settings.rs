//! Settings dialog: appearance preset, panel opacity, background image, tint and repo aliases.

use gpui::{div, prelude::*, ClickEvent, Context, PathPromptOptions};
use std::path::Path;

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
        let swatches = div().flex().gap(theme::SPACE_2).children(TINTS.iter().map(|t| {
            let t = *t;
            Swatch::new(format!("tint-{t}"), theme::parse_hex(t).unwrap_or(theme::BG)).active(s.tint.eq_ignore_ascii_case(t)).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.settings.tint = t.to_string();
                    this.save_settings(cx);
                },
            ))
        }));
        div()
            .id("settings-scroll")
            .flex()
            .flex_col()
            .gap(theme::SPACE_3)
            .overflow_y_scroll()
            .child(Field::new("Window", appearance))
            .child(Field::new("Panel opacity", opacity))
            .child(Field::new("Tint", swatches))
            .child(Field::new("Background image", self.render_bg_image_row(cx)))
            .when(s.background_image.is_some(), |d| d.child(Field::new("Image dim", dim)))
            .child(Field::new("Repo aliases", self.render_alias(cx)))
    }

    fn render_bg_image_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let name = self.settings.background_image.as_deref().map(|p| Path::new(p).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
        div()
            .flex()
            .gap(theme::SPACE_2)
            .items_center()
            .child(div().flex_1().min_w(theme::SPACE_0).truncate().text_size(theme::TEXT_XS).text_color(theme::TEXT_MUTED).child(name.clone().unwrap_or_else(|| "None".into())))
            .child(Button::new("bg-browse", "Choose...").on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                let rx = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Choose".into()) });
                cx.spawn(async move |this, cx| {
                    if let Ok(Ok(Some(paths))) = rx.await {
                        if let Some(p) = paths.into_iter().next() {
                            this.update(cx, |v, cx| {
                                v.settings.background_image = Some(p.to_string_lossy().to_string());
                                v.save_settings(cx);
                            })
                            .ok();
                        }
                    }
                })
                .detach();
            })))
            .when(name.is_some(), |d| {
                d.child(Button::new("bg-clear", "Clear").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.settings.background_image = None;
                    this.save_settings(cx);
                })))
            })
    }
}
