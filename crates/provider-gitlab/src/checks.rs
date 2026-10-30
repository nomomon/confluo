//! Pipeline jobs of the head pipeline as checks.

use gv_core::*;
use serde_json::Value;

fn job_status(j: &Value) -> CheckStatus {
    let allow = j["allow_failure"] == true;
    match j["status"].as_str().unwrap_or_default() {
        "success" => CheckStatus::Passed,
        "failed" if allow => CheckStatus::Passed,
        "failed" | "canceled" => CheckStatus::Failed,
        "running" => CheckStatus::Running,
        "skipped" => CheckStatus::Skipped,
        "manual" if allow => CheckStatus::Skipped,
        _ => CheckStatus::Pending,
    }
}

fn opt_time(v: &Value) -> Option<chrono::DateTime<chrono::Utc>> {
    v.as_str().map(|_| super::time(v))
}

/// Checks from the jobs of the head pipeline `pipeline` (`project_id/pipeline_id`).
pub(crate) fn checks_info(jobs: &[Value], pipeline: Option<String>, can_rerun: bool) -> ChecksInfo {
    let runs: Vec<CheckRun> = jobs
        .iter()
        .map(|j| {
            let status = job_status(j);
            CheckRun {
                id: j["id"].as_u64().map(|i| i.to_string()).unwrap_or_default(),
                name: super::s(&j["name"]),
                stage: j["stage"].as_str().map(str::to_string),
                status,
                started_at: opt_time(&j["started_at"]),
                finished_at: opt_time(&j["finished_at"]),
                url: j["web_url"].as_str().map(str::to_string),
                has_log: status == CheckStatus::Failed,
            }
        })
        .collect();
    let failed = runs.iter().any(|r| r.status == CheckStatus::Failed);
    ChecksInfo {
        can_rerun: can_rerun && failed && pipeline.is_some(),
        rerun_target: pipeline.filter(|_| failed),
        runs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn jobs_to_checks() {
        let jobs = vec![
            json!({"id": 1, "name": "lint", "stage": "test", "status": "failed", "web_url": "https://g/j/1", "finished_at": "2026-01-01T00:00:00Z"}),
            json!({"id": 2, "name": "allowed", "status": "failed", "allow_failure": true}),
            json!({"id": 3, "name": "deploy", "status": "manual", "allow_failure": true}),
            json!({"id": 4, "name": "build", "status": "running"}),
        ];
        let i = checks_info(&jobs, Some("9/77".into()), true);
        let st: Vec<CheckStatus> = i.runs.iter().map(|r| r.status).collect();
        assert_eq!(
            st,
            vec![
                CheckStatus::Failed,
                CheckStatus::Passed,
                CheckStatus::Skipped,
                CheckStatus::Running
            ]
        );
        assert_eq!(i.runs[0].stage.as_deref(), Some("test"));
        assert!(i.can_rerun && i.runs[0].has_log);
        assert_eq!(i.rerun_target.as_deref(), Some("9/77"));
        assert!(!checks_info(&jobs[1..], Some("9/77".into()), true).can_rerun);
    }
}
