//! UI settings persisted as JSON in the app support dir.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod general;
mod options;
pub use general::{refresh_label, Launch, OpenIn, REFRESH_MINUTES};
pub use options::{Density, MonoFont, ThemeMode, UiFont, FONT_SIZES, TINTS};

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
    /// `#rrggbb` tint mixed into the window and panels; empty for none. See `TINTS`.
    pub tint: String,
    pub theme: ThemeMode,
    pub ui_font: UiFont,
    pub mono_font: MonoFont,
    /// Body text size in px: 13, 14 or 15.
    pub font_size: u8,
    /// Spacing rhythm: Compact (default) or Comfortable.
    pub density: Density,
    /// Key of the last highlighted PR, restored as a highlight only.
    pub last_selected: Option<String>,
    /// Hide Renovate, Dependabot and other bot PRs. Read by the CLI too.
    pub hide_bots: bool,
    /// Author patterns treated as bots (`*` wildcard).
    pub hidden_authors: Vec<String>,
    /// Source branch prefixes treated as bots.
    pub hidden_branch_prefixes: Vec<String>,
    /// Auto refresh interval in minutes, 0 for manual.
    pub refresh_minutes: u32,
    pub open_in: OpenIn,
    pub launch: Launch,
    /// Project shown when the app quit (`None` for Inbox), restored with `Launch::LastView`.
    pub last_project: Option<i64>,
    /// Collapsed list sections per view (`inbox`, `starred`, `project:<id>`).
    pub collapsed_sections: std::collections::HashMap<String, Vec<crate::state::Section>>,
    /// PRs inactive longer than this many days move to the "Older" footer; `None` never. Read by the CLI.
    pub stale_days: Option<u32>,
    /// Review mode shows the diff side by side.
    pub diff_split: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: Appearance::Blurred,
            panel_opacity: 0.7,
            background_image: None,
            background_dim: 0.45,
            tint: String::new(),
            theme: ThemeMode::System,
            ui_font: UiFont::System,
            mono_font: MonoFont::JetBrainsMono,
            font_size: 13,
            density: Density::Compact,
            last_selected: None,
            hide_bots: true,
            hidden_authors: gv_engine::DEFAULT_BOT_AUTHORS.iter().map(|s| s.to_string()).collect(),
            hidden_branch_prefixes: gv_engine::DEFAULT_BOT_BRANCHES.iter().map(|s| s.to_string()).collect(),
            refresh_minutes: 5,
            open_in: OpenIn::App,
            launch: Launch::Inbox,
            last_project: None,
            collapsed_sections: Default::default(),
            stale_days: Some(gv_engine::DEFAULT_STALE_DAYS),
            diff_split: false,
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
        // Old near-black tints from before the preset swatches read as no tint.
        let tint = crate::ui::theme::parse_hex(&self.tint).filter(|c| c.l > 0.2);
        crate::ui::theme::Translucency::new(self.panel_opacity, tint, base_alpha)
    }

    /// Fonts and text step for the live theme.
    pub fn typography(&self) -> crate::ui::theme::Typography {
        let size = FONT_SIZES.iter().copied().find(|s| *s == self.font_size).unwrap_or(13);
        crate::ui::theme::Typography { ui_font: self.ui_font.family(), mono_font: self.mono_font.family(), step: f32::from(size - 13), compact: self.density == Density::Compact }
    }

    /// The bot filter the list applies.
    pub fn bot_filter(&self) -> gv_engine::BotFilter {
        gv_engine::BotFilter { enabled: self.hide_bots, authors: self.hidden_authors.clone(), branch_prefixes: self.hidden_branch_prefixes.clone() }
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
