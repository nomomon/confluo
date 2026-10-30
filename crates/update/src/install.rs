//! Download, verify and swap in a new app bundle, then relaunch it.

use crate::release::{parse_sums, Release, SUMS_ASSET};
use crate::verify::{check_signature, sha256_file, team_id};
use anyhow::{anyhow, bail, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The `.app` bundle this process runs from (`.../Confluo.app/Contents/MacOS/confluo`).
pub fn running_bundle() -> Result<PathBuf> {
    let exe = std::env::current_exe()?.canonicalize()?;
    let app = exe
        .ancestors()
        .nth(3)
        .ok_or_else(|| anyhow!("not inside an app bundle"))?;
    if app.extension().and_then(|e| e.to_str()) != Some("app") {
        bail!("not running from an app bundle");
    }
    Ok(app.to_path_buf())
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("Confluo/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

/// Stream `url` into `dest`, reporting the fraction done (0..=1) when the size is known.
async fn download(url: &str, dest: &Path, progress: &(dyn Fn(f32) + Send + Sync)) -> Result<()> {
    let mut resp = client()?.get(url).send().await?.error_for_status()?;
    let total = resp.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(dest)?;
    let mut done = 0u64;
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk)?;
        done += chunk.len() as u64;
        if total > 0 {
            progress(done as f32 / total as f32);
        }
    }
    file.flush()?;
    Ok(())
}

/// Download the release zip next to `app`, verify it and replace `app` with it.
/// Returns the path of the installed bundle (same as `app`).
pub async fn install(
    release: &Release,
    app: &Path,
    progress: &(dyn Fn(f32) + Send + Sync),
) -> Result<PathBuf> {
    let zip_name = release.zip_name();
    let zip = release
        .asset(&zip_name)
        .ok_or_else(|| anyhow!("release has no {zip_name}"))?;
    let sums = release
        .asset(SUMS_ASSET)
        .ok_or_else(|| anyhow!("release has no {SUMS_ASSET}"))?;
    let parent = app
        .parent()
        .ok_or_else(|| anyhow!("app has no parent folder"))?;
    // Same volume as the app, so the final moves are atomic renames.
    let work = parent.join(format!(".confluo-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)
        .with_context(|| format!("cannot write to {}", parent.display()))?;
    let result = async {
        let sums_text = client()?
            .get(&sums.browser_download_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let want = parse_sums(&sums_text, &zip_name)
            .ok_or_else(|| anyhow!("{zip_name} missing from {SUMS_ASSET}"))?;
        let zip_path = work.join(&zip_name);
        download(&zip.browser_download_url, &zip_path, progress).await?;
        if sha256_file(&zip_path)? != want {
            bail!("checksum mismatch for {zip_name}");
        }
        let unpacked = work.join("unpacked");
        let ok = Command::new("/usr/bin/ditto")
            .args(["-x", "-k"])
            .arg(&zip_path)
            .arg(&unpacked)
            .status()?
            .success();
        if !ok {
            bail!("could not unpack {zip_name}");
        }
        let new_app = unpacked.join(app.file_name().unwrap_or_else(|| "Confluo.app".as_ref()));
        if !new_app.is_dir() {
            bail!(
                "{zip_name} does not contain {}",
                new_app.file_name().unwrap_or_default().to_string_lossy()
            );
        }
        check_signature(&new_app, team_id(app).as_deref())?;
        swap(app, &new_app, &work)
    }
    .await;
    let _ = std::fs::remove_dir_all(&work);
    result.map(|_| app.to_path_buf())
}

/// Move `app` aside, move `new_app` into its place, then delete the old one. Rolls back on failure.
pub fn swap(app: &Path, new_app: &Path, work: &Path) -> Result<()> {
    let backup = work.join("previous.app");
    std::fs::rename(app, &backup).with_context(|| format!("moving {} aside", app.display()))?;
    if let Err(e) = std::fs::rename(new_app, app) {
        let _ = std::fs::rename(&backup, app);
        return Err(e).context("moving the new app into place");
    }
    let _ = std::fs::remove_dir_all(&backup);
    Ok(())
}

/// Open `app` as a new instance once this process has exited. The caller quits right after.
pub fn relaunch_after_exit(app: &Path) -> Result<()> {
    let script = format!(
        "while kill -0 {} 2>/dev/null; do sleep 0.2; done; /usr/bin/open -n \"$0\"",
        std::process::id()
    );
    Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .arg(app)
        .spawn()
        .context("scheduling relaunch")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swaps_bundles() {
        let root = std::env::temp_dir().join(format!("gv-update-swap-{}", std::process::id()));
        let (app, new_app, work) = (
            root.join("Confluo.app"),
            root.join("new/Confluo.app"),
            root.join("work"),
        );
        for (d, v) in [(&app, "old"), (&new_app, "new")] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join("v"), v).unwrap();
        }
        std::fs::create_dir_all(&work).unwrap();
        swap(&app, &new_app, &work).unwrap();
        assert_eq!(std::fs::read_to_string(app.join("v")).unwrap(), "new");
        assert!(!work.join("previous.app").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
