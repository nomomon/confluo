//! The one list of comment actions (registry): label, icon, menu group and an availability predicate from
//! provider capabilities, ownership and thread state. The hover toolbar, the More menu and the right-click
//! menu all read it, so they never disagree. Also the provider anchors for "Copy link to comment".

use gv_engine::ProviderKind;

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
        MENU.iter()
            .map(|g| {
                g.iter()
                    .copied()
                    .filter(|a| a.available(f))
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

/// Link to one comment: GitHub `#issuecomment-{id}` / `#discussion_r{id}` (database id decoded from the
/// node id), GitLab `#note_{id}`, Azure `?discussionId={thread}`; else the PR.
pub fn comment_link(web_url: &str, kind: ProviderKind, id: &str) -> String {
    match kind {
        ProviderKind::GitLab if id.chars().all(|c| c.is_ascii_digit()) => {
            format!("{web_url}#note_{id}")
        }
        ProviderKind::GitHub => match (id.split_once('_'), github_database_id(id)) {
            (Some(("IC", _)), Some(n)) => format!("{web_url}#issuecomment-{n}"),
            (Some(("PRRC", _)), Some(n)) => format!("{web_url}#discussion_r{n}"),
            (Some(("PRR", _)), Some(n)) => format!("{web_url}#pullrequestreview-{n}"),
            _ => web_url.to_string(),
        },
        ProviderKind::Azure => match id.split_once('/') {
            Some((t, _)) => format!("{web_url}?discussionId={t}"),
            None => web_url.to_string(),
        },
        _ => web_url.to_string(),
    }
}

/// Database id inside a GitHub node id (`IC_kwDO...`: base64url of a msgpack `[0, repo id, id]`).
pub fn github_database_id(node: &str) -> Option<u64> {
    let (_, b64) = node.split_once('_')?;
    let bytes = b64url(b64)?;
    let mut it = bytes.into_iter();
    if it.next()? & 0xf0 != 0x90 {
        return None;
    }
    let mut last = None;
    while let Some(tag) = it.next() {
        let n = match tag {
            0x00..=0x7f => tag as u64,
            0xcc => it.next()? as u64,
            0xcd => (0..2).try_fold(0u64, |a, _| Some(a << 8 | it.next()? as u64))?,
            0xce => (0..4).try_fold(0u64, |a, _| Some(a << 8 | it.next()? as u64))?,
            0xcf => (0..8).try_fold(0u64, |a, _| Some(a << 8 | it.next()? as u64))?,
            _ => return None,
        };
        last = Some(n);
    }
    last
}

fn b64url(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'-' | b'+' => Some(62),
        b'_' | b'/' => Some(63),
        _ => None,
    };
    let (mut out, mut acc, mut bits) = (vec![], 0u32, 0);
    for c in s.bytes().filter(|c| *c != b'=') {
        acc = acc << 6 | val(c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
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
    fn comment_links_per_provider() {
        let gh = "https://github.com/a/b/pull/4";
        assert_eq!(
            comment_link(gh, ProviderKind::GitHub, "IC_kwDOVCkx488AAAABhJw4UA"),
            format!("{gh}#issuecomment-6519797840")
        );
        assert_eq!(comment_link(gh, ProviderKind::GitHub, "nonsense"), gh);
        assert_eq!(
            comment_link(
                "https://gl/x/-/merge_requests/1",
                ProviderKind::GitLab,
                "42"
            ),
            "https://gl/x/-/merge_requests/1#note_42"
        );
        assert_eq!(
            comment_link(
                "https://dev.azure.com/o/p/_git/r/pullrequest/5",
                ProviderKind::Azure,
                "12/3"
            ),
            "https://dev.azure.com/o/p/_git/r/pullrequest/5?discussionId=12"
        );
        assert_eq!(quote("a\n\nb"), "> a\n>\n> b\n\n");
    }
}
