//! Avatar images: small sizes, a disk cache with a long TTL, fetched through the host governor.

use std::path::PathBuf;
use std::time::Duration;

/// Cached avatars are reused for this long.
const AVATAR_TTL: Duration = Duration::from_secs(14 * 24 * 3600);
/// A failed fetch is not retried for this long.
const MISS_TTL: Duration = Duration::from_secs(24 * 3600);
/// Avatars larger than this are ignored.
const MAX_BYTES: usize = 512 * 1024;
/// Pixel size requested from avatar hosts (2x the largest avatar drawn).
const SIZE: u32 = 64;

fn dir() -> PathBuf {
    crate::data_dir().join("avatars")
}

/// Stable FNV-1a hash for file names.
fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}

/// Ask the host for a small image: GitHub `s=`, Gravatar `s=`, GitLab uploads `width=`.
pub fn small_avatar_url(url: &str) -> String {
    let Ok(mut u) = reqwest::Url::parse(url) else {
        return url.to_string();
    };
    let host = u.host_str().unwrap_or_default().to_ascii_lowercase();
    let key = if host.ends_with("githubusercontent.com") || host.contains("gravatar.com") {
        "s"
    } else if u.path().contains("/uploads/") {
        "width"
    } else {
        return url.to_string();
    };
    let pairs: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| k != key && k != "s" && k != "width")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    u.query_pairs_mut()
        .clear()
        .extend_pairs(pairs)
        .append_pair(key, &SIZE.to_string());
    u.to_string()
}

fn fresh(p: &std::path::Path, ttl: Duration) -> bool {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age < ttl)
}

impl crate::Engine {
    /// Avatar bytes for `url`: from the disk cache when fresh, else fetched (small size) through
    /// the host governor and cached. `None` on failure; failures are remembered for a day.
    pub async fn avatar(&self, url: &str) -> Option<Vec<u8>> {
        // Demo mode never touches the network (avatar URLs are derived from invented usernames).
        if crate::demo_mode() {
            return gv_demo::avatar(url).map(<[u8]>::to_vec);
        }
        let d = dir();
        let name = format!("{:016x}", fnv(url));
        let (img, miss) = (d.join(&name), d.join(format!("{name}.miss")));
        if fresh(&img, AVATAR_TTL) {
            if let Ok(b) = std::fs::read(&img) {
                return Some(b);
            }
        }
        if fresh(&miss, MISS_TTL) {
            return std::fs::read(&img).ok();
        }
        let _ = std::fs::create_dir_all(&d);
        let client = avatar_client();
        let got = gv_http::send(client.get(small_avatar_url(url)))
            .await
            .ok()
            .filter(|r| (200..300).contains(&r.status) && r.body.len() <= MAX_BYTES);
        match got {
            Some(r) => {
                let _ = std::fs::write(&img, &r.body);
                let _ = std::fs::remove_file(&miss);
                Some(r.body)
            }
            None => {
                let _ = std::fs::write(&miss, b"");
                // A stale image is better than none.
                std::fs::read(&img).ok()
            }
        }
    }
}

fn avatar_client() -> &'static reqwest::Client {
    static C: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("Confluo")
            .timeout(Duration::from_secs(20))
            .build()
            .expect("http client")
    })
}

#[cfg(test)]
mod tests {
    use super::small_avatar_url;

    #[test]
    fn sizes_avatar_urls() {
        assert_eq!(
            small_avatar_url("https://avatars.githubusercontent.com/u/1?v=4"),
            "https://avatars.githubusercontent.com/u/1?v=4&s=64"
        );
        assert_eq!(
            small_avatar_url("https://secure.gravatar.com/avatar/abc?s=80&d=identicon"),
            "https://secure.gravatar.com/avatar/abc?d=identicon&s=64"
        );
        assert_eq!(
            small_avatar_url("https://gitlab.com/uploads/-/system/user/avatar/9/avatar.png"),
            "https://gitlab.com/uploads/-/system/user/avatar/9/avatar.png?width=64"
        );
        assert_eq!(
            small_avatar_url("https://example.com/a.png"),
            "https://example.com/a.png"
        );
    }
}
