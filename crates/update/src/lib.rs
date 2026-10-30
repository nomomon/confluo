//! Self-update from GitHub releases: check the latest release, download the app zip, verify its
//! checksum and signature, swap the bundle and relaunch.

mod install;
#[cfg(target_os = "linux")]
mod linux;
mod release;
mod verify;
mod version;

#[cfg(target_os = "macos")]
pub use install::{install, running_bundle};
pub use install::{relaunch_after_exit, swap};
#[cfg(target_os = "linux")]
pub use linux::{install, running_bundle};
pub use release::{parse_sums, Asset, Release, REPO, SUMS_ASSET};
pub use verify::{parse_team_id, sha256_file};
pub use version::{is_newer, Version};

use anyhow::{bail, Result};

/// Version of this build.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// The latest published release when it is newer than `current`. Goes through the api.github.com
/// governor with ETag revalidation, unauthenticated. A missing repo or release (404) is `None`.
pub async fn check(current: &str) -> Result<Option<Release>> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("Confluo/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let req = client
        .get(url)
        .header("Accept", "application/vnd.github+json");
    let resp = gv_http::send(req).await?;
    match resp.status {
        404 => return Ok(None),
        s if !(200..300).contains(&s) => bail!("GitHub answered {s}"),
        _ => {}
    }
    let r: Release = serde_json::from_slice(&resp.body)?;
    Ok((!r.draft && !r.prerelease && is_newer(r.version(), current)).then_some(r))
}

#[cfg(test)]
mod live {
    /// Hits the real API: `cargo test -p gv-update -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn live_check() {
        let r = super::check("0.0.1").await;
        println!(
            "live check: {:?}",
            r.as_ref().map(|o| o.as_ref().map(|x| x.tag_name.clone()))
        );
    }
}
