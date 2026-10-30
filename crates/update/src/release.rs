//! The latest GitHub release and its assets.

use serde::Deserialize;

pub const REPO: &str = "nomomon/confluo";
pub const SUMS_ASSET: &str = "SHA256SUMS";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

impl Release {
    /// Version without the leading `v`.
    pub fn version(&self) -> &str {
        self.tag_name.trim_start_matches(['v', 'V'])
    }
    /// Name of the app zip of this release: `Confluo-X.Y.Z-macos.zip`.
    pub fn zip_name(&self) -> String {
        format!("Confluo-{}-macos.zip", self.version())
    }
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == name)
    }
}

/// The SHA-256 (lowercase hex) of `file` in a `sha256sum`/`shasum -a 256` listing.
pub fn parse_sums(text: &str, file: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let mut parts = l.split_whitespace();
        let hash = parts.next()?;
        // `*name` marks binary mode in sha256sum output.
        let name = parts.next()?.trim_start_matches('*');
        (name == file && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: &str = "9F86D081884C7D659A2FEAA0C55AD015A3BF4F1B2B0B822CD15D6C15B0F00A08";

    #[test]
    fn parses_checksums() {
        let text = format!(
            "{} *Confluo-0.2.0.dmg\n{H}  Confluo-0.2.0-macos.zip\n",
            "0".repeat(64)
        );
        assert_eq!(
            parse_sums(&text, "Confluo-0.2.0-macos.zip"),
            Some(H.to_ascii_lowercase())
        );
        assert_eq!(parse_sums(&text, "Confluo-0.2.0.dmg"), Some("0".repeat(64)));
        assert_eq!(parse_sums(&text, "missing.zip"), None);
        assert_eq!(parse_sums("abc  Confluo.zip", "Confluo.zip"), None);
    }

    #[test]
    fn release_json() {
        let r: Release = serde_json::from_str(r#"{"tag_name":"v0.2.0","html_url":"https://x","assets":[{"name":"Confluo-0.2.0-macos.zip","browser_download_url":"https://d","size":3}]}"#).unwrap();
        assert_eq!(r.version(), "0.2.0");
        assert!(r.asset(&r.zip_name()).is_some());
    }
}
