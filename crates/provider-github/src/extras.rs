//! Reactions, commits, "viewed" files and my last reviewed commit, read in the detail query.

use crate::{s, time, GitHubProvider};
use gv_core::*;
use serde_json::{json, Value};

/// GraphQL selection for reactions on any `Reactable`.
pub(crate) const REACTION_FIELDS: &str = "reactionGroups { content viewerHasReacted reactors(first: 5) { totalCount nodes { ... on User { login } ... on Bot { login } } } }";

/// Extra PR fields: commits, viewed state per file and the head of each review.
pub(crate) const PR_EXTRA_FIELDS: &str = r#"
  allCommits: commits(first: 100) { nodes { commit { oid messageHeadline committedDate url author { name user { login } } } } }
  files(first: 100) { nodes { path viewerViewedState } }
"#;

pub(crate) fn reactions(v: &Value) -> Vec<ReactionGroup> {
    let mut out: Vec<ReactionGroup> = v["reactionGroups"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|g| {
            let kind = ReactionKind::parse(g["content"].as_str()?)?;
            let count = g["reactors"]["totalCount"].as_u64().unwrap_or(0) as u32;
            (count > 0).then(|| ReactionGroup {
                kind,
                count,
                me: g["viewerHasReacted"] == true,
                users: g["reactors"]["nodes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|n| n["login"].as_str().map(str::to_string))
                    .collect(),
            })
        })
        .collect();
    out.sort_by_key(|g| ReactionKind::ALL.iter().position(|k| *k == g.kind));
    out
}

pub(crate) fn commits(pr: &Value) -> Vec<CommitInfo> {
    pr["allCommits"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|n| {
            let c = &n["commit"];
            let author = c["author"]["user"]["login"]
                .as_str()
                .or_else(|| c["author"]["name"].as_str())
                .unwrap_or_default()
                .to_string();
            CommitInfo {
                sha: s(&c["oid"]),
                message: s(&c["messageHeadline"]),
                author,
                at: Some(time(&c["committedDate"])),
                url: c["url"].as_str().map(str::to_string),
            }
        })
        .collect()
}

/// `viewerViewedState` per path; only the first 100 files are known.
pub(crate) fn viewed(pr: &Value) -> Option<std::collections::HashMap<String, bool>> {
    let nodes = pr["files"]["nodes"].as_array()?;
    Some(
        nodes
            .iter()
            .filter_map(|n| {
                Some((
                    n["path"].as_str()?.to_string(),
                    n["viewerViewedState"] == "VIEWED",
                ))
            })
            .collect(),
    )
}

/// Commit my latest review was made on.
pub(crate) fn my_review_sha(pr: &Value, me: &str) -> Option<String> {
    pr["reviews"]["nodes"]
        .as_array()?
        .iter()
        .rev()
        .find(|r| r["author"]["login"] == me)
        .and_then(|r| r["commit"]["oid"].as_str())
        .map(str::to_string)
}

impl GitHubProvider {
    pub(crate) async fn do_react(
        &self,
        subject: &str,
        kind: ReactionKind,
        add: bool,
    ) -> ProviderResult<()> {
        let m = if add { "addReaction" } else { "removeReaction" };
        let q = format!("mutation($s: ID!, $c: ReactionContent!) {{ {m}(input: {{subjectId: $s, content: $c}}) {{ reaction {{ content }} }} }}");
        self.gql(&q, json!({"s": subject, "c": kind.github_graphql()}))
            .await?;
        Ok(())
    }

    pub(crate) async fn do_set_viewed(
        &self,
        pr_node: &str,
        path: &str,
        viewed: bool,
    ) -> ProviderResult<()> {
        let m = if viewed {
            "markFileAsViewed"
        } else {
            "unmarkFileAsViewed"
        };
        let q = format!("mutation($p: ID!, $f: String!) {{ {m}(input: {{pullRequestId: $p, path: $f}}) {{ clientMutationId }} }}");
        self.gql(&q, json!({"p": pr_node, "f": path})).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reaction_groups() {
        let v = json!({"reactionGroups": [
            {"content": "THUMBS_UP", "viewerHasReacted": true, "reactors": {"totalCount": 2, "nodes": [{"login": "me"}, {"login": "ada"}]}},
            {"content": "EYES", "viewerHasReacted": false, "reactors": {"totalCount": 0, "nodes": []}},
            {"content": "HEART", "viewerHasReacted": false, "reactors": {"totalCount": 1, "nodes": [{"login": "bo"}]}}
        ]});
        let r = reactions(&v);
        assert_eq!(r.len(), 2);
        assert_eq!(
            (r[0].kind, r[0].count, r[0].me),
            (ReactionKind::ThumbsUp, 2, true)
        );
        assert_eq!(r[1].users, vec!["bo".to_string()]);
    }
}
