//! UI settings persisted as JSON in the app support dir.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

impl Default for Settings {
    fn default() -> Self {
        Self { appearance: Appearance::Blurred, panel_opacity: 0.7, background_image: None, background_dim: 0.45, tint: "#14141c".into() }
    }
}

pub fn path() -> PathBuf {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
    let base = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else {
        std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| home.join(".config"))
    };
    base.join("git-viewer/settings.json")
}

impl Settings {
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
