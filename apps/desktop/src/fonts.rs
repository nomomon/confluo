//! Bundled fonts (SIL OFL 1.1), registered with GPUI at startup.

use gpui::App;
use std::borrow::Cow;

macro_rules! fonts {
    ($($path:literal),* $(,)?) => {
        &[$(include_bytes!(concat!("../assets/fonts/", $path))),*]
    };
}

const FILES: &[&[u8]] = fonts![
    "inter/Inter-Regular.ttf", "inter/Inter-Medium.ttf", "inter/Inter-SemiBold.ttf", "inter/Inter-Bold.ttf",
    "geist/Geist-Regular.ttf", "geist/Geist-Medium.ttf", "geist/Geist-SemiBold.ttf", "geist/Geist-Bold.ttf",
    "geist/GeistMono-Regular.ttf", "geist/GeistMono-Medium.ttf", "geist/GeistMono-SemiBold.ttf", "geist/GeistMono-Bold.ttf",
    "jetbrains-mono/JetBrainsMono-Regular.ttf", "jetbrains-mono/JetBrainsMono-Medium.ttf",
    "jetbrains-mono/JetBrainsMono-SemiBold.ttf", "jetbrains-mono/JetBrainsMono-Bold.ttf",
];

/// Register Inter, Geist, Geist Mono and JetBrains Mono from embedded bytes.
pub fn register(cx: &mut App) {
    let fonts = FILES.iter().map(|b| Cow::Borrowed(*b)).collect();
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        eprintln!("confluo: cannot register fonts: {e:#}");
    }
}
