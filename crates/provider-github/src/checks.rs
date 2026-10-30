//! Check runs and status contexts of the head commit, with workflow names and links.

use gv_core::*;
use serde_json::Value;

/// Extra detail-query fields: every context of the head commit's rollup.
pub(crate) const CHECK_FIELDS: &str = r#"
  checkDetail: commits(last: 1) { nodes { commit { statusCheckRollup { contexts(first: 100) { nodes {
    __typename
    ... on CheckRun { databaseId name status conclusion startedAt completedAt detailsUrl
      checkSuite { workflowRun { databaseId workflow { name } } } }
    ... on StatusContext { context state targetUrl createdAt }
  } } } } } }
"#;

fn opt_time(v: &Value) -> Option<chrono::DateTime<chrono::Utc>> {
    v.as_str().map(|_| super::time(v))
}

fn run_status(c: &Value) -> CheckStatus {
    match c["status"].as_str() {
        Some("COMPLETED") => match c["conclusion"].as_str() {
            Some("SUCCESS" | "NEUTRAL") => CheckStatus::Passed,
            Some("SKIPPED" | "STALE") => CheckStatus::Skipped,
            _ => CheckStatus::Failed,
        },
        Some("IN_PROGRESS") => CheckStatus::Running,
        _ => CheckStatus::Pending,
    }
}

fn context_status(state: &str) -> CheckStatus {
    match state {
        "SUCCESS" => CheckStatus::Passed,
        "PENDING" | "EXPECTED" => CheckStatus::Pending,
        _ => CheckStatus::Failed,
    }
}

/// Checks from the pull request node; `can_write` decides whether failed runs may be re-run.
pub(crate) fn checks_info(pr: &Value, can_write: bool) -> ChecksInfo {
    let mut runs = vec![];
    let mut rerun: Vec<String> = vec![];
    let nodes = &pr["checkDetail"]["nodes"][0]["commit"]["statusCheckRollup"]["contexts"]["nodes"];
    for c in nodes.as_array().into_iter().flatten() {
        if c["__typename"] == "CheckRun" {
            let status = run_status(c);
            let wf = &c["checkSuite"]["workflowRun"];
            if status == CheckStatus::Failed {
                if let Some(id) = wf["databaseId"]
                    .as_u64()
                    .map(|i| i.to_string())
                    .filter(|i| !rerun.contains(i))
                {
                    rerun.push(id);
                }
            }
            runs.push(CheckRun {
                id: c["databaseId"]
                    .as_u64()
                    .map(|i| i.to_string())
                    .unwrap_or_default(),
                name: super::s(&c["name"]),
                stage: wf["workflow"]["name"].as_str().map(str::to_string),
                status,
                started_at: opt_time(&c["startedAt"]),
                finished_at: opt_time(&c["completedAt"]),
                url: c["detailsUrl"].as_str().map(str::to_string),
                has_log: status == CheckStatus::Failed,
            });
        } else {
            let status = context_status(c["state"].as_str().unwrap_or_default());
            let at = opt_time(&c["createdAt"]);
            runs.push(CheckRun {
                id: String::new(),
                name: super::s(&c["context"]),
                stage: None,
                status,
                started_at: None,
                finished_at: at.filter(|_| status != CheckStatus::Pending),
                url: c["targetUrl"].as_str().map(str::to_string),
                has_log: false,
            });
        }
    }
    let rerun_target = (!rerun.is_empty()).then(|| rerun.join(","));
    ChecksInfo {
        can_rerun: can_write && rerun_target.is_some(),
        rerun_target,
        runs,
    }
}

/// Annotation lines (`path:line message`) from `GET /check-runs/{id}/annotations`.
pub(crate) fn annotation_lines(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .map(|a| {
            let msg = a["message"]
                .as_str()
                .unwrap_or_default()
                .lines()
                .next()
                .unwrap_or_default();
            match (
                a["path"]
                    .as_str()
                    .filter(|p| !p.is_empty() && *p != ".github"),
                a["start_line"].as_u64(),
            ) {
                (Some(p), Some(l)) => format!("{p}:{l} {msg}"),
                _ => msg.to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_checks() {
        let pr = json!({"checkDetail": {"nodes": [{"commit": {"statusCheckRollup": {"contexts": {"nodes": [
            {"__typename": "CheckRun", "databaseId": 11, "name": "lint", "status": "COMPLETED", "conclusion": "FAILURE",
             "startedAt": "2026-01-01T00:00:00Z", "completedAt": "2026-01-01T00:02:00Z", "detailsUrl": "https://x",
             "checkSuite": {"workflowRun": {"databaseId": 7, "workflow": {"name": "CI"}}}},
            {"__typename": "CheckRun", "databaseId": 12, "name": "test", "status": "IN_PROGRESS", "conclusion": null},
            {"__typename": "StatusContext", "context": "ci/legacy", "state": "SUCCESS", "createdAt": "2026-01-01T00:00:00Z"}
        ]}}}}]}});
        let i = checks_info(&pr, true);
        assert_eq!(i.runs.len(), 3);
        assert_eq!(i.runs[0].stage.as_deref(), Some("CI"));
        assert!(i.runs[0].has_log && i.can_rerun);
        assert_eq!(i.runs[1].status, CheckStatus::Running);
        assert_eq!(i.rerun_target.as_deref(), Some("7"));
        assert!(!checks_info(&pr, false).can_rerun);
        let a = annotation_lines(
            &json!([{"path": "src/a.rs", "start_line": 3, "message": "unused\nmore"}, {"path": ".github", "message": "exit 1"}]),
        );
        assert_eq!(a, vec!["src/a.rs:3 unused", "exit 1"]);
    }
}
