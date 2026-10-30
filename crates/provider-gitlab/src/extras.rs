//! Award emoji on the MR and its commits.

use gv_core::*;
use reqwest::Method;
use serde_json::{json, Value};

use super::{s, time, GitLabProvider};

pub(crate) fn awards(items: &[Value], me: &str) -> Vec<ReactionGroup> {
    group_reactions(items.iter().map(|a| (a["name"].as_str().unwrap_or_default(), a["user"]["username"].as_str().unwrap_or_default())), me)
}

/// `GET /merge_requests/:iid/commits` is newest first; we keep oldest first.
pub(crate) fn commits(items: &[Value]) -> Vec<CommitInfo> {
    let mut out: Vec<CommitInfo> = items
        .iter()
        .map(|c| CommitInfo { sha: s(&c["id"]), message: s(&c["title"]), author: s(&c["author_name"]), at: Some(time(&c["committed_date"])), url: c["web_url"].as_str().map(str::to_string) })
        .collect();
    out.reverse();
    out
}

impl GitLabProvider {
    /// Award emoji on the MR (`subject` empty) or on a note.
    pub(crate) async fn do_react(&self, id: &ChangeRequestId, subject: &str, kind: ReactionKind, add: bool) -> ProviderResult<()> {
        let base = Self::mr_path(id);
        let path = if subject.is_empty() { format!("{base}/award_emoji") } else { format!("{base}/notes/{subject}/award_emoji") };
        if add {
            self.call(Method::POST, &path, Some(&json!({"name": kind.gitlab()}))).await?;
            return Ok(());
        }
        let me = self.me().await?.username.clone();
        let list = self.get(&path).await?;
        let mine = list.as_array().into_iter().flatten().find(|a| a["name"] == kind.gitlab() && a["user"]["username"] == me.as_str()).and_then(|a| a["id"].as_u64());
        if let Some(award) = mine {
            self.call(Method::DELETE, &format!("{path}/{award}"), None).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_award_emoji() {
        let items = vec![json!({"name": "thumbsup", "user": {"username": "me"}}), json!({"name": "tada", "user": {"username": "ada"}}), json!({"name": "thumbsup", "user": {"username": "bo"}})];
        let g = awards(&items, "me");
        assert_eq!((g[0].kind, g[0].count, g[0].me), (ReactionKind::ThumbsUp, 2, true));
        assert_eq!((g[1].kind, g[1].me), (ReactionKind::Hooray, false));
        let c = commits(&[json!({"id": "b", "title": "two"}), json!({"id": "a", "title": "one"})]);
        assert_eq!(c[0].sha, "a");
    }
}
