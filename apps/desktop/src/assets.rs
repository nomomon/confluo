//! Embedded assets served to GPUI (`svg().path("icons/x.svg")`).

use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        &[$((concat!("icons/", $name, ".svg"), include_bytes!(concat!("../assets/icons/", $name, ".svg")))),*]
    };
}

const FILES: &[(&str, &[u8])] = icons![
    "adjustments", "alert-circle", "alert-triangle", "arrow-left", "brand-azure", "brand-github", "brand-gitlab", "brush", "bulb", "check", "chevron-down", "chevron-right", "chevron-up", "circle-check", "copy", "dots", "external-link", "eye", "eye-off", "file", "file-code", "file-text", "filter", "fold", "folder", "git-merge", "git-pull-request", "git-pull-request-closed", "git-pull-request-draft", "inbox", "info-circle", "keyboard", "layout-columns", "layout-rows", "layout-sidebar", "link", "message", "pencil", "photo", "plus", "refresh", "search", "selector", "settings", "star", "star-filled", "trash", "user", "x", "arrow-back-up", "arrow-right", "circle-x", "clock", "code", "command", "corner-up-left", "git-commit", "message-circle", "moon", "loader-2", "circle-dashed", "circle-minus", "users", "tag", "terminal",
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(FILES.iter().find(|(p, _)| *p == path).map(|(_, b)| Cow::Borrowed(*b)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(FILES.iter().filter(|(p, _)| p.starts_with(path)).map(|(p, _)| SharedString::from(*p)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_embedded() {
        for name in crate::ui::atoms::IconName::ALL.iter().copied() {
            assert!(Assets.load(name.path()).unwrap().is_some(), "{}", name.path());
        }
    }
}
