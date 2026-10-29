//! Embedded assets served to GPUI (`svg().path("icons/x.svg")`).

use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        &[$((concat!("icons/", $name, ".svg"), include_bytes!(concat!("../assets/icons/", $name, ".svg")))),*]
    };
}

const FILES: &[(&str, &[u8])] = icons![
    "arrow-left", "check", "chevron-down", "chevron-right", "chevron-up", "copy", "dots", "external-link", "file", "file-code", "file-text", "fold", "folder", "git-merge", "git-pull-request", "git-pull-request-closed", "git-pull-request-draft", "inbox", "layout-columns", "layout-rows", "layout-sidebar", "link", "message", "plus", "refresh", "search", "settings", "star", "star-filled", "x",
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
        for name in crate::ui::atoms::IconName::ALL {
            assert!(Assets.load(name.path()).unwrap().is_some(), "{}", name.path());
        }
    }
}
