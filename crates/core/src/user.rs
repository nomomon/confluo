//! How a person is shown: display names over opaque handles, teams.

use crate::User;

impl User {
    /// A team or group reviewer (`team:<slug or name>`).
    pub fn is_team(&self) -> bool {
        self.username.starts_with("team:")
    }

    /// What to show for this person: the handle, unless it is an email, a `DOMAIN\user` or a team
    /// (then the display name). A "Name <email>" display drops the email.
    pub fn label(&self) -> String {
        let name = self
            .name
            .as_deref()
            .map(clean_display)
            .filter(|n| !n.is_empty());
        let handle = self
            .username
            .strip_prefix("team:")
            .unwrap_or(&self.username);
        let opaque = self.is_team() || handle.contains('@') || handle.contains('\\');
        match name {
            Some(n) if opaque => n.to_string(),
            _ => clean_display(handle).to_string(),
        }
    }
}

/// "Ada Lovelace <ada@example.com>" -> "Ada Lovelace".
pub fn clean_display(n: &str) -> &str {
    match n.trim().split_once('<') {
        Some((name, rest)) if rest.contains('@') && !name.trim().is_empty() => name.trim(),
        _ => n.trim(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(username: &str, name: Option<&str>) -> User {
        User {
            username: username.into(),
            name: name.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn labels_prefer_display_names_over_emails() {
        assert_eq!(u("ada", Some("Ada L")).label(), "ada");
        assert_eq!(u("ada@x.org", Some("Ada L")).label(), "Ada L");
        assert_eq!(u("ada@x.org", Some("Ada L <ada@x.org>")).label(), "Ada L");
        assert_eq!(u("DOM\\ada", None).label(), "DOM\\ada");
        assert_eq!(u("ada@x.org", None).label(), "ada@x.org");
        assert_eq!(u("team:core", None).label(), "core");
        assert_eq!(u("team:[P]\\Core", Some("Core")).label(), "Core");
        assert!(u("team:core", None).is_team());
        assert_eq!(clean_display("a@b.c"), "a@b.c");
    }
}
