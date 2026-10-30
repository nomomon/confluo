//! Typography and theme choices offered in Settings.

use serde::{Deserialize, Serialize};

/// UI font family choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UiFont {
    /// SF Pro: lighter and more native than Inter in GPUI's renderer, so it is the default.
    #[default]
    System,
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
    pub const ALL: [MonoFont; 3] = [
        MonoFont::System,
        MonoFont::JetBrainsMono,
        MonoFont::GeistMono,
    ];
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

/// Spacing density; Compact is the default rhythm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Density {
    #[default]
    Compact,
    Comfortable,
}

impl Density {
    pub const ALL: [Density; 2] = [Density::Compact, Density::Comfortable];
    pub fn label(self) -> &'static str {
        match self {
            Density::Compact => "Compact",
            Density::Comfortable => "Comfortable",
        }
    }
}

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
    /// Cmd+Shift+L order: Light, Dark, System, Light.
    pub fn next(self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::System,
            ThemeMode::System => ThemeMode::Light,
        }
    }
    /// Whether the light palette applies given the window's current appearance.
    pub fn is_light(self, appearance: gpui::WindowAppearance) -> bool {
        match self {
            ThemeMode::Light => true,
            ThemeMode::Dark => false,
            ThemeMode::System => matches!(
                appearance,
                gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight
            ),
        }
    }
}

/// UI zoom steps (Cmd+= / Cmd+- / Cmd+0): text and diff rows scale by these.
pub const ZOOM_STEPS: [f32; 7] = [0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4];

/// The zoom `delta` steps away from `cur` (snapped to the nearest step), clamped to the ends.
pub fn zoom_step(cur: f32, delta: isize) -> f32 {
    let near = ZOOM_STEPS
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - cur).abs().total_cmp(&(b.1 - cur).abs()))
        .map_or(2, |(i, _)| i);
    let i = (near as isize + delta).clamp(0, ZOOM_STEPS.len() as isize - 1) as usize;
    ZOOM_STEPS[i]
}

/// Tint presets offered as swatches: name and `#rrggbb` (empty for none).
pub const TINTS: [(&str, &str); 8] = [
    ("None", ""),
    ("Slate", "#64748b"),
    ("Blue", "#3b82f6"),
    ("Violet", "#8b5cf6"),
    ("Rose", "#f43f5e"),
    ("Amber", "#f59e0b"),
    ("Emerald", "#10b981"),
    ("Teal", "#14b8a6"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_cycles_light_dark_system() {
        let mut t = ThemeMode::Light;
        let seen: Vec<_> = (0..4)
            .map(|_| {
                t = t.next();
                t
            })
            .collect();
        assert_eq!(
            seen,
            [
                ThemeMode::Dark,
                ThemeMode::System,
                ThemeMode::Light,
                ThemeMode::Dark
            ]
        );
    }

    #[test]
    fn zoom_steps_clamp_and_snap() {
        assert_eq!(zoom_step(1.0, 1), 1.1);
        assert_eq!(zoom_step(1.0, -1), 0.9);
        assert_eq!(zoom_step(1.4, 1), 1.4);
        assert_eq!(zoom_step(0.8, -1), 0.8);
        assert_eq!(zoom_step(0.8, -5), 0.8);
        assert_eq!(zoom_step(1.04, 0), 1.0);
        // A hand-edited out of range value snaps into range.
        assert_eq!(zoom_step(3.0, 0), 1.4);
        assert_eq!(zoom_step(0.1, 1), 0.9);
    }
}
