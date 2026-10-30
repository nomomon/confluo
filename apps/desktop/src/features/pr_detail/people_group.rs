//! People as an `AvatarGroup`: reviewers with their state, assignees, thread participants.

use gpui::SharedString;
use gv_engine::{ReviewState, User};

use crate::app::GitViewer;
use crate::ui::molecules::{AvatarGroup, AvatarStatus, GroupMember};

/// Lists longer than this collapse into a group in the Overview properties.
pub(super) const GROUP_ABOVE: usize = 3;

pub(super) fn status(s: ReviewState) -> Option<AvatarStatus> {
    match s {
        ReviewState::Approved => Some(AvatarStatus::Approved),
        ReviewState::ChangesRequested => Some(AvatarStatus::ChangesRequested),
        ReviewState::Commented => Some(AvatarStatus::Commented),
        ReviewState::Pending => None,
    }
}

/// Distinct users in first-seen order.
pub(super) fn distinct<'a>(users: impl IntoIterator<Item = &'a User>) -> Vec<User> {
    let mut out: Vec<User> = Vec::new();
    for u in users {
        if !out.iter().any(|o| o.username == u.username) {
            out.push(u.clone());
        }
    }
    out
}

impl GitViewer {
    pub(super) fn people_group(
        &self,
        id: impl Into<SharedString>,
        users: &[User],
        states: Option<&[ReviewState]>,
    ) -> AvatarGroup {
        let members = users
            .iter()
            .enumerate()
            .map(|(i, u)| {
                let handle = u.username.strip_prefix("team:").unwrap_or(&u.username);
                GroupMember {
                    name: u.label().into(),
                    handle: handle.to_string().into(),
                    group: u.is_team(),
                    image: self.avatar(u.avatar_url.as_deref()),
                    status: states.and_then(|s| s.get(i)).and_then(|s| status(*s)),
                }
            })
            .collect();
        AvatarGroup::new(id, members)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(name: &str) -> User {
        User {
            username: name.into(),
            name: None,
            avatar_url: None,
            bot: false,
            id: None,
        }
    }

    #[test]
    fn distinct_keeps_first_seen_order() {
        let v = [u("a"), u("b"), u("a"), u("c")];
        let d: Vec<_> = distinct(&v).into_iter().map(|x| x.username).collect();
        assert_eq!(d, ["a", "b", "c"]);
        assert_eq!(status(ReviewState::Pending), None);
    }
}
