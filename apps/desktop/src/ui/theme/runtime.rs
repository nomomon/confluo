//! Live theme state: light/dark palette, font families and UI text step, set by the app each
//! frame from the settings and read by the token functions.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

/// User-chosen typography applied through the theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Typography {
    pub ui_font: &'static str,
    pub mono_font: &'static str,
    /// Points added to every text size (0 for the 13px body default).
    pub step: f32,
    /// Compact density (default) tightens the rhythm tokens; false is Comfortable.
    pub compact: bool,
}

impl Default for Typography {
    fn default() -> Self {
        Self { ui_font: ".SystemUIFont", mono_font: "JetBrains Mono", step: 0., compact: true }
    }
}

static TYPOGRAPHY: RwLock<Typography> = RwLock::new(Typography { ui_font: ".SystemUIFont", mono_font: "JetBrains Mono", step: 0., compact: true });

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

static LIGHT: AtomicBool = AtomicBool::new(false);

/// Switch the semantic palette between light and dark.
pub fn set_light(light: bool) {
    LIGHT.store(light, Ordering::Relaxed);
}

/// Whether the light palette is active.
pub fn is_light() -> bool {
    LIGHT.load(Ordering::Relaxed)
}
