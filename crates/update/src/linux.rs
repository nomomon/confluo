//! Linux self-update: only the AppImage replaces itself. `.deb` and tarball installs get a link.

use crate::install::{fetch_verified, work_dir};
use crate::release::Release;
use anyhow::{bail, Context, Result};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// The AppImage this process runs from (`$APPIMAGE`, set by the AppImage runtime).
pub fn running_bundle() -> Result<PathBuf> {
    appimage_path(std::env::var_os("APPIMAGE"))
}

fn appimage_path(var: Option<std::ffi::OsString>) -> Result<PathBuf> {
    match var.filter(|v| !v.is_empty()) {
        Some(p) => Ok(PathBuf::from(p)),
        None => bail!("installed by a package manager, update from the release page"),
    }
}

/// Download the release AppImage, verify its checksum and atomically replace `appimage` with it.
pub async fn install(
    release: &Release,
    appimage: &Path,
    progress: &(dyn Fn(f32) + Send + Sync),
) -> Result<PathBuf> {
    let work = work_dir(appimage)?;
    let result = async {
        let new = fetch_verified(release, &release.appimage_name(), &work, progress).await?;
        std::fs::set_permissions(&new, std::fs::Permissions::from_mode(0o755))?;
        std::fs::rename(&new, appimage).with_context(|| format!("replacing {}", appimage.display()))
    }
    .await;
    let _ = std::fs::remove_dir_all(&work);
    result.map(|_| appimage.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appimage_only_from_env() {
        assert!(appimage_path(None).is_err());
        assert!(appimage_path(Some("".into())).is_err());
        assert_eq!(
            appimage_path(Some("/opt/Confluo.AppImage".into())).unwrap(),
            PathBuf::from("/opt/Confluo.AppImage")
        );
    }
}
