//! Checks a downloaded update must pass: checksum, a valid signature and the same team.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::process::Command;

/// Lowercase hex SHA-256 of a file.
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Team ID from `codesign -dv` output; None when ad-hoc or unsigned.
pub fn parse_team_id(codesign_dv: &str) -> Option<String> {
    let id = codesign_dv.lines().find_map(|l| l.trim().strip_prefix("TeamIdentifier="))?.trim();
    (!id.is_empty() && id != "not set").then(|| id.to_string())
}

/// Team ID the bundle at `app` is signed with (None when ad-hoc or unsigned).
pub fn team_id(app: &Path) -> Option<String> {
    let out = Command::new("/usr/bin/codesign").arg("-dv").arg(app).output().ok()?;
    // codesign prints the details on stderr.
    parse_team_id(&String::from_utf8_lossy(&out.stderr))
}

/// `codesign --verify --deep --strict`, and when `expected_team` is set, the same Team ID.
pub fn check_signature(app: &Path, expected_team: Option<&str>) -> Result<()> {
    let out = Command::new("/usr/bin/codesign").args(["--verify", "--deep", "--strict"]).arg(app).output().context("running codesign")?;
    if !out.status.success() {
        bail!("signature check failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    if let Some(want) = expected_team {
        match team_id(app) {
            Some(got) if got == want => {}
            got => bail!("signed by team {} instead of {want}", got.as_deref().unwrap_or("none")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_team_ids() {
        let dev = "Executable=/Applications/Confluo.app/Contents/MacOS/confluo\nIdentifier=com.nomomon.confluo\nAuthority=Developer ID Application: Someone (AB12CD34EF)\nTeamIdentifier=AB12CD34EF\nSealed Resources version=2\n";
        assert_eq!(parse_team_id(dev).as_deref(), Some("AB12CD34EF"));
        assert_eq!(parse_team_id("Signature=adhoc\nTeamIdentifier=not set\n"), None);
        assert_eq!(parse_team_id("code object is not signed at all"), None);
    }

    #[test]
    fn hashes_files() {
        let p = std::env::temp_dir().join(format!("gv-update-hash-{}", std::process::id()));
        std::fs::write(&p, b"test").unwrap();
        assert_eq!(sha256_file(&p).unwrap(), "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08");
        let _ = std::fs::remove_file(&p);
    }
}
