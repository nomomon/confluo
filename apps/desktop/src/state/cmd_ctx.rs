//! What the keymap commands may do right now: a plain snapshot of the window (PR open, Review mode,
//! capabilities) that `keymap::Entry::available` predicates and the gated PR actions read.

use gv_engine::{EditCapabilities, ReviewCapabilities};

/// Capabilities of the open PR once its detail has loaded.
#[derive(Clone, Default, Debug)]
pub struct Gate {
    pub review: ReviewCapabilities,
    pub edit: EditCapabilities,
}

/// Snapshot of the window for command availability.
#[derive(Clone, Default, Debug)]
pub struct CmdCtx {
    pub pr_open: bool,
    pub reviewing: bool,
    /// The diff of the open PR is showing (Review mode with files).
    pub diff: bool,
    pub projects: usize,
    pub gate: Option<Gate>,
    /// The whole-file viewer is open (Ctrl+G).
    pub file_view: bool,
    /// Draft state of the open PR (`None` without one).
    pub draft: Option<bool>,
}

/// Label of the draft action for the current state; the neutral one when no PR is open.
pub fn draft_label(draft: Option<bool>) -> &'static str {
    match draft {
        Some(true) => "Ready for review",
        Some(false) => "Mark as draft",
        None => DRAFT_LABEL,
    }
}

/// Keymap label of the draft action (no PR in context).
pub const DRAFT_LABEL: &str = "Mark as draft or ready for review";

/// `label` of keymap `action`, specialised to the current state.
pub fn live_label(action: &str, label: &'static str, draft: Option<bool>) -> &'static str {
    if action == "ToggleDraft" {
        draft_label(draft)
    } else {
        label
    }
}

/// PR actions gated by the provider's capabilities.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrAct {
    Approve,
    RequestChanges,
    Merge,
    ToggleDraft,
    Close,
}

impl CmdCtx {
    /// Why `a` is not possible now (shown as a toast), or `None` when it is.
    pub fn deny(&self, a: PrAct) -> Option<String> {
        if !self.pr_open {
            return Some("Open a pull request first".into());
        }
        let Some(g) = &self.gate else {
            return Some("Still loading the pull request".into());
        };
        let (r, e) = (&g.review, &g.edit);
        let why = match a {
            PrAct::Approve if r.approve => return None,
            PrAct::Approve if r.unapprove => "You already approved this pull request".to_string(),
            PrAct::Approve => r
                .approve_blocked
                .clone()
                .unwrap_or_else(|| "Approving isn't possible here".into()),
            PrAct::RequestChanges if r.request_changes => return None,
            PrAct::RequestChanges
                if r.approve_blocked
                    .as_deref()
                    .is_some_and(|w| w.contains("own")) =>
            {
                "You can't request changes on your own PR".into()
            }
            PrAct::RequestChanges if r.any() => "This provider has no request changes".into(),
            PrAct::RequestChanges => "This pull request isn't open".into(),
            PrAct::Merge if e.merge => return None,
            PrAct::Merge => e
                .merge_blocked
                .clone()
                .unwrap_or_else(|| "You can't merge this pull request".into()),
            PrAct::ToggleDraft if e.draft => return None,
            PrAct::ToggleDraft => "You can't change the draft state of this pull request".into(),
            PrAct::Close if e.close || e.reopen => return None,
            PrAct::Close => "You can't close this pull request".into(),
        };
        Some(why)
    }

    pub fn allows(&self, a: PrAct) -> bool {
        self.deny(a).is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(review: ReviewCapabilities, edit: EditCapabilities) -> CmdCtx {
        CmdCtx {
            pr_open: true,
            gate: Some(Gate { review, edit }),
            ..Default::default()
        }
    }

    #[test]
    fn no_pr_or_loading_denies_everything() {
        assert_eq!(
            CmdCtx::default().deny(PrAct::Approve).as_deref(),
            Some("Open a pull request first")
        );
        let loading = CmdCtx {
            pr_open: true,
            ..Default::default()
        };
        assert!(!loading.allows(PrAct::Merge));
    }

    #[test]
    fn draft_label_follows_state() {
        assert_eq!(draft_label(Some(true)), "Ready for review");
        assert_eq!(draft_label(Some(false)), "Mark as draft");
        assert_eq!(live_label("ToggleDraft", DRAFT_LABEL, None), DRAFT_LABEL);
        assert_eq!(live_label("Approve", "Approve", Some(true)), "Approve");
    }

    #[test]
    fn own_pr_explains_why() {
        let r = ReviewCapabilities {
            comment_review: true,
            approve_blocked: Some("You can't approve your own PR".into()),
            ..Default::default()
        };
        let c = ctx(r, EditCapabilities::editor(true));
        assert_eq!(
            c.deny(PrAct::Approve).as_deref(),
            Some("You can't approve your own PR")
        );
        assert_eq!(
            c.deny(PrAct::RequestChanges).as_deref(),
            Some("You can't request changes on your own PR")
        );
        assert!(c.allows(PrAct::ToggleDraft));
        assert!(c.allows(PrAct::Close));
        assert!(!c.allows(PrAct::Merge));
    }

    #[test]
    fn reviewer_can_review_but_not_edit() {
        let r = ReviewCapabilities {
            approve: true,
            request_changes: true,
            comment_review: true,
            ..Default::default()
        };
        let e = EditCapabilities {
            merge: true,
            ..Default::default()
        };
        let c = ctx(r, e);
        assert!(
            c.allows(PrAct::Approve) && c.allows(PrAct::RequestChanges) && c.allows(PrAct::Merge)
        );
        assert!(!c.allows(PrAct::ToggleDraft));
        let blocked = ctx(
            ReviewCapabilities::default(),
            EditCapabilities {
                merge_blocked: Some("Has conflicts".into()),
                ..Default::default()
            },
        );
        assert_eq!(blocked.deny(PrAct::Merge).as_deref(), Some("Has conflicts"));
    }
}
