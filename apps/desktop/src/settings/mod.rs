//! UI settings persisted as JSON in the app support dir.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod options;
pub use options::{MonoFont, UiFont, FONT_SIZES};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Appearance {
    #[default]
    Blurred,
    Transparent,
    Opaque,
}

impl Appearance {
    pub const ALL: [Appearance; 3] = [Appearance::Blurred, Appearance::Transparent, Appearance::Opaque];
    pub fn label(&self) -> &'static str {
        match self {
            Appearance::Blurred => "Blurred",
            Appearance::Transparent => "Transparent",
            Appearance::Opaque => "Opaque",
        }
    }
    pub fn to_gpui(self) -> gpui::WindowBackgroundAppearance {
        match self {
            Appearance::Blurred => gpui::WindowBackgroundAppearance::Blurred,
            Appearance::Transparent => gpui::WindowBackgroundAppearance::Transparent,
            Appearance::Opaque => gpui::WindowBackgroundAppearance::Opaque,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub appearance: Appearance,
    /// 0.2..=1.0 alpha of sidebar, list and detail panels.
    pub panel_opacity: f32,
    /// Optional image drawn behind the whole window.
    pub background_image: Option<String>,
    /// Dim overlay over the background image, 0..=1.
    pub background_dim: f32,
    /// `#rrggbb` tint drawn behind the panels.
    pub tint: String,
    pub ui_font: UiFont,
    pub mono_font: MonoFont,
    /// Body text size in px: 13, 14 or 15.
    pub font_size: u8,
    /// Key of the last highlighted PR, restored as a highlight only.
    pub last_selected: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: Appearance::Blurred,
            panel_opacity: 0.7,
            background_image: None,
            background_dim: 0.45,
            tint: "#14141c".into(),
            ui_font: UiFont::Inter,
            mono_font: MonoFont::JetBrainsMono,
            font_size: 13,
            last_selected: None,
        }
    }
}

pub fn path() -> PathBuf {
    gv_engine::data_dir().join("settings.json")
}

impl Settings {
    /// Panel, card and window fills derived from the appearance.
    pub fn surfaces(&self) -> crate::ui::theme::Translucency {
        let base_alpha = match self.appearance {
            Appearance::Opaque => 1.0,
            Appearance::Blurred => 0.35,
            Appearance::Transparent => 0.55,
        };
        crate::ui::theme::Translucency::new(self.panel_opacity, crate::ui::theme::parse_hex(&self.tint), base_alpha)
    }

    /// Fonts and text step for the live theme.
    pub fn typography(&self) -> crate::ui::theme::Typography {
        let size = FONT_SIZES.iter().copied().find(|s| *s == self.font_size).unwrap_or(13);
        crate::ui::theme::Typography { ui_font: self.ui_font.family(), mono_font: self.mono_font.family(), step: f32::from(size - 13) }
    }

    pub fn load() -> Self {
        std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let p = path();
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(p, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}
