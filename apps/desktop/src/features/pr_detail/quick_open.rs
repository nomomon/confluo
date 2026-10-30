//! Cmd+P in Review mode: jump to a changed file by name (fuzzy).

use gpui::{prelude::*, AnyElement, Context, SharedString};

use crate::app::GitViewer;
use crate::state::DetailState;
use crate::ui::atoms::IconName;
use crate::ui::molecules::command_palette::score;
use crate::ui::molecules::{CommandPalette, PaletteItem};

impl GitViewer {
    /// File indices matching the picker query, best first.
    fn quick_matches(&self, cx: &Context<Self>) -> Vec<usize> {
        let Some(files) = self.detail.as_ref().and_then(|d| d.code.files.clone()) else { return vec![] };
        let q = self.quick_input.read(cx).text();
        let mut hits: Vec<(i32, usize)> = files.iter().enumerate().filter_map(|(i, f)| score(&q, f.path()).map(|s| (s, i))).collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        hits.into_iter().map(|(_, i)| i).collect()
    }

    pub(crate) fn run_quick_file(&mut self, sel: usize, cx: &mut Context<Self>) {
        let hit = self.quick_matches(cx).get(sel).copied();
        self.review.quick_open = false;
        self.refocus = true;
        if let (Some(fi), Some(d)) = (hit, self.detail.as_mut()) {
            if d.code.collapsed.remove(&fi) {
                d.code.rebuild();
            }
            d.code.goto(fi);
        }
        cx.notify()
    }

    pub(super) fn render_quick_open(&self, d: &DetailState, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.review.quick_open {
            return None;
        }
        let files = d.code.files.clone()?;
        let items: Vec<PaletteItem> = self
            .quick_matches(cx)
            .into_iter()
            .map(|i| {
                let p = files[i].path();
                let (dir, name) = p.rsplit_once('/').unwrap_or(("", p));
                PaletteItem { icon: IconName::FileCode, title: SharedString::from(name.to_string()), detail: Some(SharedString::from(dir.to_string())), shortcut: None, group: "Files" }
            })
            .collect();
        let sel = self.review.quick_sel.min(items.len().saturating_sub(1));
        Some(
            CommandPalette::new(self.quick_input.clone())
                .items(items)
                .selected(sel)
                .on_move(cx.listener(|this, delta: &isize, _, cx| {
                    let n = this.quick_matches(cx).len();
                    if n > 0 {
                        this.review.quick_sel = (this.review.quick_sel as isize + delta).rem_euclid(n as isize) as usize;
                    }
                    cx.notify()
                }))
                .on_run(cx.listener(|this, i: &usize, _, cx| this.run_quick_file(*i, cx)))
                .on_close(cx.listener(|this, _: &(), _, cx| {
                    this.review.quick_open = false;
                    this.refocus = true;
                    cx.notify()
                }))
                .into_any_element(),
        )
    }
}
