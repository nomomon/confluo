//! Typography and theme choices offered in Settings.

use serde::{Deserialize, Serialize};

/// UI font family choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UiFont {
    System,
    #[default]
    Inter,
    Geist,
}

impl UiFont {
    pub const ALL: [UiFont; 3] = [UiFont::System, UiFont::Inter, UiFont::Geist];
    pub fn label(self) -> &'static str {
        match self {
            UiFont::System => "System",
            UiFont::Inter => "Inter",
            UiFont::Geist => "Geist",
        }
    }
    pub fn family(self) -> &'static str {
        match self {
            UiFont::System => ".SystemUIFont",
            UiFont::Inter => "Inter",
            UiFont::Geist => "Geist",
        }
    }
}

/// Monospace font family choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonoFont {
    System,
    #[default]
    JetBrainsMono,
    GeistMono,
}

impl MonoFont {
    pub const ALL: [MonoFont; 3] = [MonoFont::System, MonoFont::JetBrainsMono, MonoFont::GeistMono];
    pub fn label(self) -> &'static str {
        match self {
            MonoFont::System => "Menlo",
            MonoFont::JetBrainsMono => "JetBrains Mono",
            MonoFont::GeistMono => "Geist Mono",
        }
    }
    pub fn family(self) -> &'static str {
        match self {
            MonoFont::System => "Menlo",
            MonoFont::JetBrainsMono => "JetBrains Mono",
            MonoFont::GeistMono => "Geist Mono",
        }
    }
}

/// Body text sizes offered; the theme adds `size - 13` to every step.
pub const FONT_SIZES: [u8; 3] = [13, 14, 15];

/// Color theme; `System` follows the macOS appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 3] = [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark];
    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::System => "System",
            ThemeMode::Light => "Light",
            ThemeMode::Dark => "Dark",
        }
    }
    /// Whether the light palette applies given the window's current appearance.
    pub fn is_light(self, appearance: gpui::WindowAppearance) -> bool {
        match self {
            ThemeMode::Light => true,
            ThemeMode::Dark => false,
            ThemeMode::System => matches!(appearance, gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight),
        }
    }
}
