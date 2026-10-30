//! Self-update: periodic checks for a newer GitHub release, download with progress, swap and relaunch.

use gpui::Context;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use super::{bridge, timing};
use crate::app::GitViewer;

#[derive(Default)]
pub(crate) struct UpdateState {
    /// Newer release offered in the sidebar.
    pub available: Option<gv_update::Release>,
    /// Download progress 0..=1 while installing.
    pub progress: Option<f32>,
    pub checking: bool,
}

/// Whether a found release is shown: a manual check always shows it, otherwise not when dismissed.
fn should_offer(version: &str, dismissed: Option<&str>, manual: bool) -> bool {
    manual || dismissed != Some(version)
}

impl GitViewer {
    /// Check on launch and every `UPDATE_EVERY` while automatic checks are on. Never in demo mode.
    pub(crate) fn start_update_checks(&mut self, cx: &mut Context<Self>) {
        if let Some(v) = crate::dev::fake_update() {
            let html_url = format!("https://github.com/{}/releases/tag/v{v}", gv_update::REPO);
            self.update.available = Some(gv_update::Release {
                tag_name: format!("v{v}"),
                html_url,
                draft: false,
                prerelease: false,
                assets: vec![],
            });
            self.update.progress = crate::dev::fake_update_progress();
            return;
        }
        if gv_engine::demo_mode() {
            return;
        }
        cx.spawn(async move |this, cx| loop {
            let alive = this.update(cx, |v, cx| {
                if v.settings.auto_update {
                    v.check_for_updates(false, cx);
                }
            });
            if alive.is_err() {
                break;
            }
            cx.background_executor().timer(timing::UPDATE_EVERY).await;
        })
        .detach();
    }

    /// Ask GitHub for the latest release. Automatic checks fail silently; manual ones say what happened.
    pub(crate) fn check_for_updates(&mut self, manual: bool, cx: &mut Context<Self>) {
        if self.update.checking || self.update.progress.is_some() {
            return;
        }
        self.update.checking = true;
        cx.notify();
        let rx = bridge::spawn(gv_update::check(gv_update::CURRENT));
        cx.spawn(async move |this, cx| {
            let res = rx
                .await
                .unwrap_or_else(|_| Err(anyhow::anyhow!("check cancelled")));
            let _ = this.update(cx, |v, cx| {
                v.update.checking = false;
                match res {
                    Ok(Some(r))
                        if should_offer(
                            r.version(),
                            v.settings.dismissed_update.as_deref(),
                            manual,
                        ) =>
                    {
                        v.update.available = Some(r)
                    }
                    Ok(Some(_)) => {}
                    Ok(None) if manual => v.show_info_toast(
                        format!("Confluo {} is up to date", gv_update::CURRENT),
                        cx,
                    ),
                    Ok(None) => {}
                    Err(e) if manual => {
                        v.show_info_toast(format!("Couldn't check for updates: {e:#}"), cx)
                    }
                    Err(e) => eprintln!("confluo: update check failed: {e:#}"),
                }
                cx.notify()
            });
        })
        .detach();
    }

    /// Hide the card until a newer version than this one appears.
    pub(crate) fn dismiss_update(&mut self, cx: &mut Context<Self>) {
        if let Some(r) = self.update.available.take() {
            self.settings.dismissed_update = Some(r.version().to_string());
            self.save_settings(cx);
        }
        cx.notify();
    }

    /// Download, verify and install the offered release, then relaunch into it.
    pub(crate) fn install_update(&mut self, cx: &mut Context<Self>) {
        let Some(release) = self.update.available.clone() else {
            return;
        };
        if self.update.progress.is_some() {
            return;
        }
        let app = match gv_update::running_bundle() {
            Ok(a) => a,
            Err(e) => return self.show_info_toast(format!("Can't update: {e:#}"), cx),
        };
        self.update.progress = Some(0.);
        let done = Arc::new(AtomicU32::new(0f32.to_bits()));
        let report = done.clone();
        let rx = bridge::spawn(async move {
            gv_update::install(&release, &app, &move |f: f32| {
                report.store(f.to_bits(), Ordering::Relaxed)
            })
            .await
        });
        cx.spawn(async move |this, cx| {
            let mut rx = rx;
            let res = loop {
                match rx.try_recv() {
                    Ok(Some(r)) => break r,
                    Err(_) => break Err(anyhow::anyhow!("update task stopped")),
                    Ok(None) => {}
                }
                cx.background_executor()
                    .timer(timing::UPDATE_PROGRESS)
                    .await;
                let p = f32::from_bits(done.load(Ordering::Relaxed));
                if this
                    .update(cx, |v, cx| {
                        v.update.progress = Some(p);
                        cx.notify()
                    })
                    .is_err()
                {
                    return;
                }
            };
            let _ = this.update(cx, |v, cx| {
                v.update.progress = None;
                match res.and_then(|app| gv_update::relaunch_after_exit(&app)) {
                    Ok(()) => cx.quit(),
                    Err(e) => v.show_info_toast(format!("Update failed: {e:#}"), cx),
                }
                cx.notify()
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::should_offer;

    #[test]
    fn dismissed_versions_stay_hidden_unless_asked() {
        assert!(should_offer("0.2.0", None, false));
        assert!(!should_offer("0.2.0", Some("0.2.0"), false));
        assert!(should_offer("0.3.0", Some("0.2.0"), false));
        assert!(should_offer("0.2.0", Some("0.2.0"), true));
    }
}
