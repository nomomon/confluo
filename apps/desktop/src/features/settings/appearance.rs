//! Appearance: theme, window material, panel opacity, tint, background image and typography.

use gpui::{div, img, prelude::*, ClickEvent, Context, ObjectFit, PathPromptOptions};
use std::path::{Path, PathBuf};

use crate::app::GitViewer;
use crate::features::layout;
use crate::settings::{Appearance, Density, MonoFont, ThemeMode, UiFont, FONT_SIZES, TINTS};
use crate::ui::atoms::{tooltip, Button, ButtonVariant, Swatch};
use crate::ui::molecules::{Field, Select, Slider, ToggleGroup};
use crate::ui::theme;

impl GitViewer {
    pub(super) fn render_appearance(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let s = &self.settings;
        let mode = ThemeMode::ALL.iter().fold(ToggleGroup::new("theme"), |g, m| g.item(m.label(), None, *m == s.theme)).on_change(cx.listener(
            |this, i: &usize, _, cx| {
                this.settings.theme = ThemeMode::ALL[*i];
                this.save_settings(cx)
            },
        ));
        let material = Appearance::ALL.iter().fold(ToggleGroup::new("material"), |g, a| g.item(a.label(), None, *a == s.appearance)).on_change(cx.listener(
            |this, i: &usize, _, cx| {
                this.settings.appearance = Appearance::ALL[*i];
                this.save_settings(cx)
            },
        ));
        let opacity = Slider::new("panel-opacity", s.panel_opacity).range(0., 1.).step(0.05).on_change(cx.listener(|this, v: &f32, _, cx| {
            this.settings.panel_opacity = *v;
            this.save_settings(cx)
        }));
        let swatches = div().flex().gap(theme::SPACE_2).children(TINTS.iter().map(|(name, hex)| {
            let (name, hex) = (*name, *hex);
            let color = theme::parse_hex(hex).unwrap_or_else(theme::surface_active);
            let active = s.tint.eq_ignore_ascii_case(hex) || (hex.is_empty() && theme::parse_hex(&s.tint).is_none_or(|c| c.l <= 0.2));
            div().id(gpui::SharedString::from(format!("tint-tip-{name}"))).tooltip(tooltip(name)).child(Swatch::new(format!("tint-{name}"), color).active(active).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.settings.tint = hex.to_string();
                    this.save_settings(cx)
                },
            )))
        }));
        let ui_font = UiFont::ALL
            .iter()
            .fold(Select::new("ui-font"), |sel, f| sel.font_option(f.label(), f.family()))
            .selected(UiFont::ALL.iter().position(|f| *f == s.ui_font).unwrap_or(0))
            .on_change(cx.listener(|this, i: &usize, _, cx| {
                this.settings.ui_font = UiFont::ALL[*i];
                this.save_settings(cx)
            }));
        let mono_font = MonoFont::ALL
            .iter()
            .fold(Select::new("mono-font"), |sel, f| sel.font_option(f.label(), f.family()))
            .selected(MonoFont::ALL.iter().position(|f| *f == s.mono_font).unwrap_or(0))
            .on_change(cx.listener(|this, i: &usize, _, cx| {
                this.settings.mono_font = MonoFont::ALL[*i];
                this.save_settings(cx)
            }));
        let size = FONT_SIZES.iter().fold(ToggleGroup::new("font-size"), |g, v| g.item(format!("{v}"), None, *v == s.font_size)).on_change(cx.listener(
            |this, i: &usize, _, cx| {
                this.settings.font_size = FONT_SIZES[*i];
                this.save_settings(cx)
            },
        ));
        let density = Density::ALL.iter().fold(ToggleGroup::new("density"), |g, d| g.item(d.label(), None, *d == s.density)).on_change(cx.listener(
            |this, i: &usize, _, cx| {
                this.settings.density = Density::ALL[*i];
                this.save_settings(cx)
            },
        ));
        let mut image_rows = vec![Field::new("Background image", self.render_bg_image(cx)).description("Drawn behind the window, under the panels").horizontal().into_any_element()];
        if s.background_image.is_some() {
            let dim = Slider::new("bg-dim", s.background_dim).range(0., 0.9).step(0.05).on_change(cx.listener(|this, v: &f32, _, cx| {
                this.settings.background_dim = *v;
                this.save_settings(cx)
            }));
            image_rows.push(Field::new("Image dim", dim).description("Darken the image so text stays readable").horizontal().into_any_element());
        }
        div()
            .child(self.settings_group(
                Some("Window"),
                vec![
                    Field::new("Theme", mode).description("System follows the macOS appearance").horizontal().into_any_element(),
                    Field::new("Material", material).description("Blurred and Transparent show the desktop behind").horizontal().into_any_element(),
                    Field::new("Panel opacity", opacity).description("Sidebar and detail panels").horizontal().into_any_element(),
                    Field::new("Tint", swatches).description("Colors the window and panels").horizontal().into_any_element(),
                ],
            ))
            .child(self.settings_group(Some("Background"), image_rows))
            .child(self.settings_group(
                Some("Text"),
                vec![
                    Field::new("Interface font", ui_font).horizontal().into_any_element(),
                    Field::new("Code font", mono_font).description("Diffs, branches and commands").horizontal().into_any_element(),
                    Field::new("Text size", size).horizontal().into_any_element(),
                    Field::new("Density", density).description("Spacing of lists, cards and controls").horizontal().into_any_element(),
                ],
            ))
    }

    fn render_bg_image(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let path = self.settings.background_image.clone().filter(|p| Path::new(p).is_file());
        let thumb = div()
            .w(layout::SETTINGS_THUMB_W)
            .h(layout::SETTINGS_THUMB_H)
            .rounded(theme::RADIUS_MD)
            .overflow_hidden()
            .border_1()
            .border_color(theme::border())
            .bg(theme::surface())
            .when_some(path.clone(), |d, p| d.child(img(PathBuf::from(p)).size_full().object_fit(ObjectFit::Cover)));
        div()
            .flex()
            .items_center()
            .gap(theme::SPACE_2)
            .child(thumb)
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
            .when(path.is_some(), |d| {
                d.child(Button::new("bg-clear", "Remove").variant(ButtonVariant::Ghost).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.settings.background_image = None;
                    this.save_settings(cx);
                })))
            })
    }
}
