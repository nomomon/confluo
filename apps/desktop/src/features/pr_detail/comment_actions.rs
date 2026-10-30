//! The one list of comment actions (registry): label, icon, menu group and an availability predicate from
//! provider capabilities, ownership and thread state. The hover toolbar, the More menu and the right-click
//! menu all read it, so they never disagree.

use crate::ui::atoms::IconName;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentAction {
    Reply,
    QuoteReply,
    Resolve,
    Unresolve,
    Edit,
    Delete,
    CopyText,
    CopyLink,
    OpenInBrowser,
    ShowInCode,
}

/// What decides availability.
#[derive(Clone, Copy, Debug, Default)]
pub struct CommentFacts {
    pub mine: bool,
    pub pending: bool,
    /// The comment's thread takes replies.
    pub can_reply: bool,
    /// Thread resolve state when it can be resolved.
    pub resolved: Option<bool>,
    /// Inline (diff) thread with a line to show.
    pub inline: bool,
    /// Shown in the Conversation (Show in code applies there).
    pub in_conversation: bool,
}

/// Menu order; groups are separated (destructive last within its group).
pub const MENU: [&[CommentAction]; 4] = [
    &[CommentAction::Reply, CommentAction::QuoteReply],
    &[CommentAction::Resolve, CommentAction::Unresolve],
    &[
        CommentAction::CopyText,
        CommentAction::CopyLink,
        CommentAction::OpenInBrowser,
        CommentAction::ShowInCode,
    ],
    &[CommentAction::Edit, CommentAction::Delete],
];

/// Actions with their own button in the hover toolbar (besides reactions and More).
pub const TOOLBAR: [CommentAction; 3] = [
    CommentAction::Reply,
    CommentAction::Resolve,
    CommentAction::Unresolve,
];

impl CommentAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Reply => "Reply",
            Self::QuoteReply => "Quote reply",
            Self::Resolve => "Resolve thread",
            Self::Unresolve => "Unresolve thread",
            Self::Edit => "Edit",
            Self::Delete => "Delete",
            Self::CopyText => "Copy text",
            Self::CopyLink => "Copy link to comment",
            Self::OpenInBrowser => "Open in browser",
            Self::ShowInCode => "Show in code",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Self::Reply => IconName::CornerUpLeft,
            Self::QuoteReply => IconName::MessageCircle,
            Self::Resolve => IconName::Check,
            Self::Unresolve => IconName::ArrowBackUp,
            Self::Edit => IconName::Pencil,
            Self::Delete => IconName::Trash,
            Self::CopyText => IconName::Copy,
            Self::CopyLink => IconName::Link,
            Self::OpenInBrowser => IconName::ExternalLink,
            Self::ShowInCode => IconName::Code,
        }
    }

    pub fn destructive(self) -> bool {
        self == Self::Delete
    }

    pub fn available(self, f: &CommentFacts) -> bool {
        if f.pending {
            return false;
        }
        match self {
            Self::Reply | Self::QuoteReply => f.can_reply,
            Self::Resolve => f.resolved == Some(false),
            Self::Unresolve => f.resolved == Some(true),
            Self::Edit | Self::Delete => f.mine,
            Self::CopyText | Self::CopyLink | Self::OpenInBrowser => true,
            Self::ShowInCode => f.inline && f.in_conversation,
        }
    }

    /// Available actions in menu groups (empty groups dropped).
    pub fn menu(f: &CommentFacts) -> Vec<Vec<CommentAction>> {
        Self::menu_without(f, &[])
    }

    /// The hover toolbar's More menu: only what the toolbar does not already show (Slack rule).
    pub fn more_menu(f: &CommentFacts) -> Vec<Vec<CommentAction>> {
        Self::menu_without(f, &TOOLBAR)
    }

    fn menu_without(f: &CommentFacts, skip: &[CommentAction]) -> Vec<Vec<CommentAction>> {
        MENU.iter()
            .map(|g| {
                g.iter()
                    .copied()
                    .filter(|a| a.available(f) && !skip.contains(a))
                    .collect::<Vec<_>>()
            })
            .filter(|g| !g.is_empty())
            .collect()
    }
}

/// `> ` quoted body plus a blank line, for Quote reply.
pub fn quote(body: &str) -> String {
    let q: Vec<String> = body
        .lines()
        .map(|l| {
            if l.is_empty() {
                ">".into()
            } else {
                format!("> {l}")
            }
        })
        .collect();
    format!("{}\n\n", q.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_follows_ownership_and_thread_state() {
        let mine = CommentFacts {
            mine: true,
            can_reply: true,
            resolved: Some(false),
            ..Default::default()
        };
        let menu = CommentAction::menu(&mine);
        assert_eq!(
            menu.first().unwrap(),
            &vec![CommentAction::Reply, CommentAction::QuoteReply]
        );
        assert_eq!(
            menu.last().unwrap(),
            &vec![CommentAction::Edit, CommentAction::Delete]
        );
        assert!(
            CommentAction::Resolve.available(&mine) && !CommentAction::Unresolve.available(&mine)
        );
        let theirs = CommentFacts {
            mine: false,
            ..Default::default()
        };
        assert!(
            !CommentAction::Delete.available(&theirs) && !CommentAction::Reply.available(&theirs)
        );
        assert!(CommentAction::menu(&CommentFacts {
            pending: true,
            ..mine
        })
        .is_empty());
        assert!(CommentAction::ShowInCode.available(&CommentFacts {
            inline: true,
            in_conversation: true,
            ..Default::default()
        }));
    }

    #[test]
    fn more_menu_skips_toolbar_actions() {
        let f = CommentFacts {
            can_reply: true,
            resolved: Some(false),
            mine: true,
            ..Default::default()
        };
        let more: Vec<CommentAction> = CommentAction::more_menu(&f).concat();
        assert!(TOOLBAR.iter().all(|a| !more.contains(a)));
        assert!(more.contains(&CommentAction::QuoteReply) && more.contains(&CommentAction::Edit));
        let full: Vec<CommentAction> = CommentAction::menu(&f).concat();
        assert!(full.contains(&CommentAction::Reply) && full.contains(&CommentAction::Resolve));
    }

    #[test]
    fn quotes_body() {
        assert_eq!(quote("a\n\nb"), "> a\n>\n> b\n\n");
    }
}
