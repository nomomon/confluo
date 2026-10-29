//! Live theme state: font families and UI text step, set by the app each
//! frame from the settings and read by the token functions.

use std::sync::RwLock;

/// User-chosen typography applied through the theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Typography {
    pub ui_font: &'static str,
    pub mono_font: &'static str,
    /// Points added to every text size (0 for the 13px body default).
    pub step: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self { ui_font: "Inter", mono_font: "JetBrains Mono", step: 0. }
    }
}

static TYPOGRAPHY: RwLock<Typography> = RwLock::new(Typography { ui_font: "Inter", mono_font: "JetBrains Mono", step: 0. });

/// Replace the live typography; the next render picks it up.
pub fn set_typography(t: Typography) {
    if let Ok(mut g) = TYPOGRAPHY.write() {
        *g = t;
    }
}

pub fn typography() -> Typography {
    TYPOGRAPHY.read().map(|g| *g).unwrap_or_default()
}

/// UI font family.
pub fn ui_font() -> &'static str {
    typography().ui_font
}

/// Monospace family for branches, commands and SHAs.
pub fn mono_font() -> &'static str {
    typography().mono_font
}
