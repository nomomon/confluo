//! Demo avatars: invented, served from the binary (no network). The bot's logo has a transparent
//! background, to check that avatars sit on a solid disc.

const LINTBOT_SVG: &[u8] = include_bytes!("fixtures/lintbot.svg");
const PREFIX: &str = "demo-avatar:";

/// Avatar URL of a demo user, if it has one.
pub fn url_for(username: &str) -> Option<String> {
    (username == "lintbot").then(|| format!("{PREFIX}{username}"))
}

/// Bytes behind a demo avatar URL.
pub fn avatar(url: &str) -> Option<&'static [u8]> {
    match url.strip_prefix(PREFIX)? {
        "lintbot" => Some(LINTBOT_SVG),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_logo_is_served() {
        let url = url_for("lintbot").unwrap();
        assert!(avatar(&url).unwrap().starts_with(b"<svg"));
        assert!(url_for("ada").is_none() && avatar("https://x").is_none());
    }
}
