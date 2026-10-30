//! Whole-file viewer and per-file totals derived from the loaded diff.

use std::sync::Arc;

use gpui::{Context, UniformListScrollHandle};
use gv_engine::{ChangeRequest, ChangeRequestId, ChangedFile};

use super::{bridge, err_str};
use crate::app::GitViewer;

/// Whole-file viewer opened from a file card's "View file".
pub struct FileView {
    pub path: String,
    pub lines: Option<Arc<Vec<String>>>,
    pub error: Option<String>,
    pub scroll: UniformListScrollHandle,
    /// Selected lines (0-based indices).
    pub sel: Option<super::line_select::LineRange>,
}

impl GitViewer {
    /// GitLab lists carry no line counts; fill them from the loaded files.
    pub(super) fn apply_file_totals(&mut self, id: &ChangeRequestId, files: &[ChangedFile]) {
        let (a, del) = gv_engine::totals(files);
        let n = files.len() as u32;
        let fill = |c: &mut ChangeRequest| {
            c.additions = Some(a);
            c.deletions = Some(del);
            c.changed_files = Some(n);
        };
        self.items.iter_mut().filter(|c| &c.id == id).for_each(fill);
        if let Some(d) = self.detail.as_mut().filter(|d| &d.summary.id == id) {
            fill(&mut d.summary);
        }
    }

    /// Fetch and show the whole file at the head sha.
    pub(crate) fn open_file_view(&mut self, path: String, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        let id = d.summary.id.clone();
        let sha = d
            .detail
            .as_ref()
            .and_then(|x| x.head_sha.clone())
            .unwrap_or_default();
        d.code.viewer = Some(FileView {
            path: path.clone(),
            lines: None,
            error: None,
            scroll: UniformListScrollHandle::new(),
            sel: None,
        });
        let engine = self.engine.clone();
        let p = path.clone();
        let rx =
            bridge::spawn(async move { engine.file_content(&id, &p, &sha).await.map_err(err_str) });
        cx.spawn(async move |this, cx| {
            let Ok(r) = rx.await else { return };
            this.update(cx, |v, cx| {
                let Some(fv) = v
                    .detail
                    .as_mut()
                    .and_then(|d| d.code.viewer.as_mut())
                    .filter(|f| f.path == path)
                else {
                    return;
                };
                match r {
                    Ok(Some(c)) => {
                        fv.lines = Some(Arc::new(c.lines().map(str::to_string).collect()))
                    }
                    Ok(None) => fv.error = Some("File not found at the head commit".into()),
                    Err(e) => fv.error = Some(e),
                }
                cx.notify()
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
