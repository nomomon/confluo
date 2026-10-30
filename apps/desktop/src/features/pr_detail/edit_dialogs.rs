//! Confirm dialogs for closing a PR and re-running checks, and the merge dialog.

use gpui::{div, prelude::*, ClickEvent, Context};
use gv_engine::{MergeMethod, ProviderKind};

use crate::app::{GitViewer, Overlay};
use crate::ui::atoms::{Button, ButtonVariant, Checkbox};
use crate::ui::molecules::{Dialog, RadioGroup};
use crate::ui::theme;

impl GitViewer {
    fn noun(&self) -> &'static str {
        match self.detail.as_ref().map(|d| d.summary.id.repo.kind) {
            Some(ProviderKind::GitLab) => "merge request",
            _ => "pull request",
        }
    }

    fn dialog_footer(&self, id: &'static str, label: String, destructive: bool, cx: &mut Context<Self>, on_ok: impl Fn(&mut Self, &mut Context<Self>) + 'static) -> impl IntoElement {
        div()
            .flex()
            .gap(theme::SPACE_2)
            .child(Button::new("edit-dialog-cancel", "Cancel").variant(ButtonVariant::Ghost).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.overlay = Overlay::None;
                this.refocus = true;
                cx.notify()
            })))
            .child(Button::new(id, label).variant(if destructive { ButtonVariant::Destructive } else { ButtonVariant::Primary }).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| on_ok(this, cx))))
    }

    pub(crate) fn render_close_dialog(&self, cx: &mut Context<Self>) -> Dialog {
        let title = self.detail.as_ref().map(|d| d.summary.title.clone()).unwrap_or_default();
        Dialog::new(format!("Close this {}?", self.noun()))
            .description(format!("\"{title}\" is closed without merging. You can reopen it later."))
            .content(div())
            .footer(self.dialog_footer("close-confirm", "Close".into(), true, cx, |this, cx| this.set_open(false, cx)))
    }

    pub(crate) fn render_delete_comment_dialog(&self, cx: &mut Context<Self>) -> Dialog {
        Dialog::new("Delete comment?").description("This can't be undone.").content(div()).footer(self.dialog_footer("delete-comment-confirm", "Delete".into(), true, cx, |this, cx| this.delete_comment(cx)))
    }

    pub(crate) fn render_rerun_dialog(&self, cx: &mut Context<Self>) -> Dialog {
        let n = self.detail.as_ref().and_then(|d| d.detail.as_ref()).map(|x| x.checks.count(gv_engine::CheckStatus::Failed)).unwrap_or(0);
        Dialog::new("Re-run failed checks?")
            .description(format!("{n} failed {} start again on the same commit.", if n == 1 { "check" } else { "checks" }))
            .content(div())
            .footer(self.dialog_footer("rerun-confirm", "Re-run".into(), false, cx, |this, cx| this.rerun_failed(cx)))
    }

    pub(crate) fn render_merge_dialog(&self, cx: &mut Context<Self>) -> Dialog {
        let Some(d) = self.detail.as_ref() else { return Dialog::new("Merge") };
        let caps = d.detail.as_ref().map(|x| x.edit.caps.clone()).unwrap_or_default();
        let o = self.merge_opts;
        let methods = caps.merge_methods.clone();
        let mut content = div().flex().flex_col().gap(theme::SPACE_3).text_size(theme::text_sm());
        if methods.len() > 1 {
            let mut radios = RadioGroup::new("merge-method").selected(methods.iter().position(|m| *m == o.method).unwrap_or(0));
            for m in &methods {
                radios = radios.option(m.label());
            }
            let ms = methods.clone();
            content = content.child(radios.on_change(cx.listener(move |this, i: &usize, _, cx| {
                this.merge_opts.method = ms[*i];
                cx.notify()
            })));
        }
        let check = |id: &'static str, label: &'static str, on: bool, set: fn(&mut gv_engine::MergeOptions, bool), cx: &mut Context<Self>| {
            Checkbox::new(id, on).label(label).on_change(cx.listener(move |this, v: &bool, _, cx| {
                set(&mut this.merge_opts, *v);
                cx.notify()
            }))
        };
        if caps.squash_option {
            content = content.child(check("merge-squash", "Squash commits", o.squash, |m, v| m.squash = v, cx));
        }
        if caps.delete_branch_option {
            content = content.child(check("merge-delete", "Delete source branch", o.delete_branch, |m, v| m.delete_branch = v, cx));
        }
        if caps.auto_merge {
            content = content.child(check("merge-auto", "Merge when the pipeline succeeds", o.when_pipeline_succeeds, |m, v| m.when_pipeline_succeeds = v, cx));
        }
        let label = if o.when_pipeline_succeeds {
            "Set to auto-merge".to_string()
        } else {
            match o.method {
                MergeMethod::Squash => "Squash and merge".into(),
                MergeMethod::Rebase => "Rebase and merge".into(),
                MergeMethod::RebaseMerge => "Rebase and merge".into(),
                MergeMethod::Merge if o.squash => "Squash and merge".into(),
                MergeMethod::Merge => "Merge".into(),
            }
        };
        let s = &d.summary;
        Dialog::new(format!("Merge this {}?", self.noun()))
            .description(format!("{} into {}. This can't be undone from Confluo.", s.source_branch, s.target_branch))
            .content(content)
            .footer(self.dialog_footer("merge-confirm", label, true, cx, |this, cx| this.submit_merge(cx)))
    }
}
