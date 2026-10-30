//! Demo mode (`CONFLUO_DEMO=1`): fixture providers, a throwaway data dir, no network.

use crate::{Engine, Source};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Whether `CONFLUO_DEMO` is set (to anything but `0` or empty).
pub fn demo_mode() -> bool {
    std::env::var("CONFLUO_DEMO").is_ok_and(|v| !v.is_empty() && v != "0")
}

/// Data dir of demo mode: settings, cache and avatars live here, never in the user's dir.
pub fn demo_dir() -> PathBuf {
    let d = std::env::temp_dir().join("confluo-demo");
    let _ = std::fs::create_dir_all(&d);
    d
}

impl Engine {
    /// A fresh demo engine: the demo dir is wiped, fixture providers are installed for
    /// github.com and gitlab.com, and the demo projects are created.
    pub fn open_demo() -> Result<Self> {
        let dir = demo_dir();
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        let engine = Self::open_with(&dir.join("cache.db"), vec![gv_demo::GITLAB.into()])?;
        {
            let mut map = engine.providers.try_lock().expect("new engine is not shared");
            for (kind, host) in [(crate::ProviderKind::GitHub, gv_demo::GITHUB), (crate::ProviderKind::GitLab, gv_demo::GITLAB)] {
                map.insert((kind, host.into()), Arc::new(gv_demo::DemoProvider::new(kind, host)));
            }
        }
        for (name, kind, host, group) in gv_demo::PROJECTS {
            engine.add_source(name, Source::Group { provider: *kind, host: (*host).into(), path: (*group).into() })?;
        }
        // Seed the cache so the first frame (and dev hooks such as `CONFLUO_OPEN`) see the PRs.
        engine.store.upsert_change_requests(&gv_demo::change_requests())?;
        for host in [gv_demo::GITHUB, gv_demo::GITLAB] {
            let kind = if host == gv_demo::GITHUB { crate::ProviderKind::GitHub } else { crate::ProviderKind::GitLab };
            engine.remember_me(kind, host, gv_demo::ME);
        }
        Ok(engine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RefreshTarget;

    #[tokio::test]
    async fn demo_engine_refreshes_offline() {
        let e = Engine::open_demo().unwrap();
        let r = e.refresh(RefreshTarget::InvolvingMe).await.unwrap();
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert!(r.fetched > 5);
        let p = e.project("Orbit").unwrap();
        let r = e.refresh(RefreshTarget::Project(p.id)).await.unwrap();
        assert!(r.errors.is_empty() && r.fetched >= 4, "{r:?}");
    }
}
