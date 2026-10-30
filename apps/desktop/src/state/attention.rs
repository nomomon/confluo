//! Thread attention in the open PR: Conversation filter, unresolved counts and "next unresolved".

use gpui::Context;
use gv_engine::{threads_attention, Attention};

use super::{DetailState, RightTab};
use crate::app::GitViewer;

/// Conversation filter.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThreadFilter {
    #[default]
    All,
    Unresolved,
    NeedsMe,
}

impl ThreadFilter {
    pub fn keeps(self, a: Attention) -> bool {
        match self {
            Self::All => true,
            Self::Unresolved => a.unresolved(),
            Self::NeedsMe => a == Attention::NeedsMe,
        }
    }
}

/// Attention of every thread of the open PR, in thread order (empty while loading).
pub fn attention(d: &DetailState) -> Vec<Attention> {
    d.detail
        .as_ref()
        .map(|det| threads_attention(det, d.me.as_deref()))
        .unwrap_or_default()
}

/// The thread `dir` steps (1 or -1) from `at` among the unresolved ones, wrapping; needs-me threads
/// come first so the most urgent are visited first.
pub fn step_unresolved(
    ids: &[(String, Attention)],
    at: Option<&str>,
    dir: isize,
) -> Option<String> {
    let mut order: Vec<&(String, Attention)> = ids.iter().filter(|(_, a)| a.unresolved()).collect();
    order.sort_by_key(|(_, a)| *a != Attention::NeedsMe);
    if order.is_empty() {
        return None;
    }
    let n = order.len() as isize;
    let next = match at.and_then(|id| order.iter().position(|(x, _)| x == id)) {
        Some(i) => (i as isize + dir).rem_euclid(n),
        None if dir > 0 => 0,
        None => n - 1,
    };
    Some(order[next as usize].0.clone())
}

impl GitViewer {
    /// `u` / Shift+U: the next or previous unresolved thread, in the diff when it sits on a current
    /// line, else in the Conversation panel.
    pub(crate) fn jump_unresolved(&mut self, dir: isize, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_ref() else {
            return;
        };
        let Some(det) = d.detail.as_ref() else { return };
        let ids: Vec<(String, Attention)> = det
            .threads
            .iter()
            .map(|t| t.id.clone())
            .zip(attention(d))
            .collect();
        let Some(id) = step_unresolved(&ids, self.review.attn_at.as_deref(), dir) else {
            return self.show_info_toast("No unresolved threads", cx);
        };
        let in_diff = det
            .threads
            .iter()
            .find(|t| t.id == id)
            .is_some_and(|t| t.position.is_some() && !t.outdated);
        self.review.attn_at = Some(id.clone());
        // The Conversation panel shows the thread too, so both sides land on it.
        self.review.right_tab = RightTab::Conversation;
        if !self.review.thread_filter.keeps(Attention::Open) {
            self.review.thread_filter = ThreadFilter::Unresolved;
        }
        *self.review.reveal_conv.borrow_mut() = Some(id.clone());
        if in_diff {
            self.show_thread_in_code(id, cx);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_through_unresolved_needs_me_first() {
        let ids = vec![
            ("a".to_string(), Attention::Open),
            ("b".into(), Attention::Resolved),
            ("c".into(), Attention::NeedsMe),
            ("d".into(), Attention::Outdated),
        ];
        assert_eq!(step_unresolved(&ids, None, 1).as_deref(), Some("c"));
        assert_eq!(step_unresolved(&ids, Some("c"), 1).as_deref(), Some("a"));
        assert_eq!(step_unresolved(&ids, Some("d"), 1).as_deref(), Some("c"));
        assert_eq!(step_unresolved(&ids, Some("c"), -1).as_deref(), Some("d"));
        assert_eq!(
            step_unresolved(&[("b".into(), Attention::Resolved)], None, 1),
            None
        );
        assert!(
            ThreadFilter::NeedsMe.keeps(Attention::NeedsMe)
                && !ThreadFilter::Unresolved.keeps(Attention::Resolved)
        );
    }
}
