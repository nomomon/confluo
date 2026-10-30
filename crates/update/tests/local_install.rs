//! End to end install from a locally served `dist/` (see docs/releasing.md):
//! `python3 -m http.server 8765 -d dist &` then
//! `GV_UPDATE_BASE=http://127.0.0.1:8765 GV_UPDATE_APP=/tmp/x/Confluo.app GV_UPDATE_VERSION=0.1.0 cargo test -p gv-update --test local_install -- --ignored`

use gv_update::{Asset, Release};

#[tokio::test]
#[ignore]
async fn installs_from_local_dist() {
    let (Ok(base), Ok(app), Ok(v)) = (std::env::var("GV_UPDATE_BASE"), std::env::var("GV_UPDATE_APP"), std::env::var("GV_UPDATE_VERSION")) else { return };
    let asset = |name: String| Asset { browser_download_url: format!("{base}/{name}"), name, size: 0 };
    let r = Release { tag_name: format!("v{v}"), html_url: base.clone(), draft: false, prerelease: false, assets: vec![asset(format!("Confluo-{v}-macos.zip")), asset("SHA256SUMS".into())] };
    let last = std::sync::Mutex::new(0f32);
    let installed = gv_update::install(&r, std::path::Path::new(&app), &|p| *last.lock().unwrap() = p).await.unwrap();
    assert!(installed.join("Contents/MacOS/confluo").exists());
    assert_eq!(*last.lock().unwrap(), 1.0);
}
