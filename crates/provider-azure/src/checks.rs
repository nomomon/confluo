//! Checks from branch policy evaluations (build validation, reviewers, comments) and PR statuses.

use gv_core::*;
use serde_json::Value;

use crate::parse::{opt_time, seg};

const BUILD_POLICY: &str = "0609b952-1397-4640-95ec-e00a01b2c241";

fn is_build(e: &Value) -> bool {
    let t = &e["configuration"]["type"];
    t["id"].as_str() == Some(BUILD_POLICY) || t["displayName"].as_str() == Some("Build")
}

fn policy_status(e: &Value) -> CheckStatus {
    match e["status"].as_str().unwrap_or_default() {
        // An expired build result has to be queued again before completing.
        "approved" if e["context"]["isExpired"] == true => CheckStatus::Pending,
        "approved" => CheckStatus::Passed,
        "rejected" | "broken" => CheckStatus::Failed,
        "running" => CheckStatus::Running,
        "queued" => CheckStatus::Pending,
        _ => CheckStatus::Skipped,
    }
}

fn policy_name(e: &Value) -> String {
    let c = &e["configuration"];
    let kind = c["type"]["displayName"].as_str().unwrap_or("Policy");
    let named = c["settings"]["displayName"]
        .as_str()
        .or(e["context"]["buildDefinitionName"].as_str())
        .filter(|n| !n.is_empty());
    match (is_build(e), named) {
        (true, Some(n)) => n.to_string(),
        (true, None) => format!(
            "Build {}",
            c["settings"]["buildDefinitionId"]
                .as_u64()
                .map(|i| i.to_string())
                .unwrap_or_default()
        )
        .trim()
        .to_string(),
        (false, _) => kind.to_string(),
    }
}

/// One check per enabled policy evaluation.
pub(crate) fn policy_runs(evals: &[Value], org: &str, project: &str) -> Vec<CheckRun> {
    evals
        .iter()
        .filter(|e| {
            e["configuration"]["isEnabled"] != false && e["configuration"]["isDeleted"] != true
        })
        .map(|e| {
            let status = policy_status(e);
            let build = e["context"]["buildId"].as_u64();
            CheckRun {
                id: e["evaluationId"].as_str().unwrap_or_default().to_string(),
                name: policy_name(e),
                stage: Some(
                    if e["configuration"]["isBlocking"] == false {
                        "Optional policy"
                    } else {
                        "Required policy"
                    }
                    .into(),
                ),
                status,
                started_at: opt_time(&e["startedDate"]),
                finished_at: opt_time(&e["completedDate"]),
                url: build.map(|b| {
                    format!(
                        "https://dev.azure.com/{}/{}/_build/results?buildId={b}",
                        seg(org),
                        seg(project)
                    )
                }),
                has_log: false,
            }
        })
        .collect()
}

/// PR statuses posted by external services; the latest per context.
pub(crate) fn status_runs(statuses: &[Value]) -> Vec<CheckRun> {
    let mut latest: Vec<&Value> = vec![];
    for st in statuses {
        let key = |v: &Value| {
            (
                v["context"]["genre"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                v["context"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            )
        };
        match latest.iter_mut().find(|x| key(x) == key(st)) {
            Some(x) if st["id"].as_u64() >= x["id"].as_u64() => *x = st,
            Some(_) => {}
            None => latest.push(st),
        }
    }
    latest
        .into_iter()
        .map(|st| {
            let status = match st["state"].as_str().unwrap_or_default() {
                "succeeded" => CheckStatus::Passed,
                "failed" | "error" => CheckStatus::Failed,
                "pending" => CheckStatus::Running,
                "notApplicable" => CheckStatus::Skipped,
                _ => CheckStatus::Pending,
            };
            let ctx = &st["context"];
            let name = match (
                ctx["genre"].as_str().filter(|g| !g.is_empty()),
                ctx["name"].as_str().filter(|n| !n.is_empty()),
            ) {
                (Some(g), Some(n)) => format!("{g}/{n}"),
                (None, Some(n)) => n.to_string(),
                _ => st["description"].as_str().unwrap_or("status").to_string(),
            };
            let done = !matches!(status, CheckStatus::Running | CheckStatus::Pending);
            CheckRun {
                id: String::new(),
                name,
                stage: Some("Status".into()),
                status,
                started_at: opt_time(&st["creationDate"]),
                finished_at: if done {
                    opt_time(&st["updatedDate"]).or(opt_time(&st["creationDate"]))
                } else {
                    None
                },
                url: st["targetUrl"].as_str().map(str::to_string),
                has_log: false,
            }
        })
        .collect()
}

pub(crate) fn summary(runs: &[CheckRun]) -> CheckSummary {
    let mut c = CheckSummary::default();
    for r in runs.iter().filter(|r| r.status != CheckStatus::Skipped) {
        c.total += 1;
        match r.status {
            CheckStatus::Passed => c.passed += 1,
            CheckStatus::Failed => c.failed += 1,
            _ => c.pending += 1,
        }
    }
    c
}

/// Approvals the "Minimum number of reviewers" policy asks for (0 without one).
pub(crate) fn required_approvers(evals: &[Value]) -> u32 {
    evals
        .iter()
        .filter(|e| e["configuration"]["isEnabled"] != false)
        .filter_map(|e| e["configuration"]["settings"]["minimumApproverCount"].as_u64())
        .max()
        .unwrap_or(0) as u32
}

/// Evaluation ids of failed or expired build policies (they can be queued again).
pub(crate) fn failed_builds(evals: &[Value]) -> Vec<String> {
    evals
        .iter()
        .filter(|e| {
            is_build(e)
                && (matches!(e["status"].as_str(), Some("rejected" | "broken"))
                    || e["context"]["isExpired"] == true)
        })
        .filter_map(|e| e["evaluationId"].as_str().map(str::to_string))
        .collect()
}

/// Whether a blocking policy is failing or still running.
pub(crate) fn blocking(evals: &[Value]) -> (bool, bool) {
    let blocking = evals.iter().filter(|e| {
        e["configuration"]["isBlocking"] != false && e["configuration"]["isEnabled"] != false
    });
    let (mut failed, mut running) = (false, false);
    for e in blocking {
        match policy_status(e) {
            CheckStatus::Failed => failed = true,
            CheckStatus::Running | CheckStatus::Pending => running = true,
            _ => {}
        }
    }
    (failed, running)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evals() -> Vec<Value> {
        let v: Value =
            serde_json::from_str(include_str!("../fixtures/policy_evaluations.json")).unwrap();
        v["value"].as_array().unwrap().clone()
    }

    #[test]
    fn policies_to_checks() {
        let runs = policy_runs(&evals(), "contoso", "Fabrikam Web");
        let got: Vec<(&str, CheckStatus)> =
            runs.iter().map(|r| (r.name.as_str(), r.status)).collect();
        assert_eq!(
            got,
            vec![
                ("CI", CheckStatus::Failed),
                ("e2e", CheckStatus::Running),
                ("Minimum number of reviewers", CheckStatus::Passed),
                ("Comment requirements", CheckStatus::Pending),
                ("Work item linking", CheckStatus::Skipped),
            ]
        );
        assert_eq!(
            runs[0].url.as_deref(),
            Some("https://dev.azure.com/contoso/Fabrikam%20Web/_build/results?buildId=901")
        );
        assert_eq!(runs[1].stage.as_deref(), Some("Optional policy"));
        assert!(runs[0].finished_at.is_some());
        assert_eq!(
            summary(&runs),
            CheckSummary {
                total: 4,
                passed: 1,
                failed: 1,
                pending: 2
            }
        );
        assert_eq!(required_approvers(&evals()), 2);
        assert_eq!(failed_builds(&evals()), vec!["ev-build-1"]);
        assert_eq!(blocking(&evals()), (true, true));
    }

    #[test]
    fn statuses_keep_latest() {
        let v: Value = serde_json::from_str(include_str!("../fixtures/statuses.json")).unwrap();
        let runs = status_runs(v["value"].as_array().unwrap());
        assert_eq!(runs.len(), 2);
        assert_eq!(
            (runs[0].name.as_str(), runs[0].status),
            ("security/scan", CheckStatus::Passed)
        );
        assert_eq!(
            runs[0].url.as_deref(),
            Some("https://scanner.example.com/r/2")
        );
        assert_eq!(
            (runs[1].name.as_str(), runs[1].status),
            ("coverage", CheckStatus::Failed)
        );
    }
}
