//! Invented people of the demo fixture.

use gv_core::User;

/// The demo viewer.
pub const ME: &str = "mira";

const PEOPLE: &[(&str, &str)] = &[
    ("mira", "Mira Sol"),
    ("ada", "Ada Park"),
    ("linus-k", "Linus Kearney"),
    ("grace.h", "Grace Holm"),
    ("tomas", "Tomas Vidal"),
    ("priya.r", "Priya Raman"),
    ("noor", "Noor Haddad"),
];

/// A person by username; unknown names become bots (e.g. `renovate[bot]`).
pub fn user(username: &str) -> User {
    let name = PEOPLE.iter().find(|(u, _)| *u == username).map(|(_, n)| n.to_string());
    let bot = name.is_none();
    User { username: username.into(), name: name.or_else(|| Some(username.into())), avatar_url: None, bot, id: Some(username.into()) }
}

/// Everyone matching `query` (for the reviewer picker).
pub fn search(query: &str) -> Vec<User> {
    let q = query.to_lowercase();
    PEOPLE.iter().filter(|(u, n)| u.contains(&q) || n.to_lowercase().contains(&q)).map(|(u, _)| user(u)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bots_and_people() {
        assert!(!user("ada").bot);
        assert_eq!(user("ada").name.as_deref(), Some("Ada Park"));
        assert!(user("renovate[bot]").bot);
        assert_eq!(search("gra").len(), 1);
    }
}
