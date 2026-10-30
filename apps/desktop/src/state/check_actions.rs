//! Checks card actions: expand a check (lazy log tail for failures) and re-run failed jobs.

use gpui::Context;

use super::pr_edit::LogState;
use super::{bridge, err_str};
use crate::app::{GitViewer, Overlay};

impl GitViewer {
    pub(crate) fn toggle_show_passed(&mut self, cx: &mut Context<Self>) {
        if let Some(d) = self.detail.as_mut() {
            d.ui.show_passed = !d.ui.show_passed;
        }
        cx.notify();
    }

    /// Expand or collapse a check; the first expand of a failed one fetches its log tail.
    pub(crate) fn toggle_check(&mut self, check_id: String, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        if !d.ui.open_checks.remove(&check_id) {
            d.ui.open_checks.insert(check_id.clone());
        }
        let run = d
            .detail
            .as_ref()
            .and_then(|x| x.checks.runs.iter().find(|r| r.id == check_id))
            .cloned();
        let Some(run) = run.filter(|r| r.has_log && !d.ui.logs.contains_key(&check_id)) else {
            return cx.notify();
        };
        d.ui.logs.insert(check_id.clone(), LogState::Loading);
        let (engine, id, want) = (
            self.engine.clone(),
            d.summary.id.clone(),
            d.summary.id.clone(),
        );
        let rx = bridge::spawn(async move { engine.check_log(&id, &run).await.map_err(err_str) });
        cx.spawn(async move |this, cx| {
            let r = rx.await.unwrap_or_else(|_| Err("cancelled".into()));
            this.update(cx, |v, cx| {
                if let Some(d) = v.detail.as_mut().filter(|d| d.summary.id == want) {
                    let state = match r {
                        Ok(lines) => LogState::Lines(lines),
                        Err(e) => LogState::Failed(e),
                    };
                    d.ui.logs.insert(check_id, state);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn confirm_rerun(&mut self, cx: &mut Context<Self>) {
        self.overlay = Overlay::ConfirmRerun;
        cx.notify();
    }

    pub(crate) fn rerun_failed(&mut self, cx: &mut Context<Self>) {
        self.overlay = Overlay::None;
        let Some(d) = self.detail.as_ref() else {
            return;
        };
        let (engine, id) = (self.engine.clone(), d.summary.id.clone());
        self.run_edit(
            "Re-running failed checks",
            "Re-running checks",
            |_| {},
            async move { engine.rerun_failed(&id).await },
            cx,
        );
    }
}
