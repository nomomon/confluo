//! Individual checks of the head commit (GitHub check runs and statuses, GitLab jobs).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CheckStatus {
    Failed,
    Running,
    /// Queued, created, waiting for a runner or a manual action.
    Pending,
    Passed,
    Skipped,
}

impl CheckStatus {
    /// Sort order: failing and running first.
    pub fn rank(&self) -> u8 {
        match self {
            Self::Failed => 0,
            Self::Running => 1,
            Self::Pending => 2,
            Self::Passed => 3,
            Self::Skipped => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRun {
    /// GitHub check run database id or GitLab job id; empty for status contexts.
    pub id: String,
    pub name: String,
    /// GitLab stage or GitHub workflow name.
    pub stage: Option<String>,
    pub status: CheckStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub url: Option<String>,
    /// Whether a log tail or annotations can be fetched for it.
    #[serde(default)]
    pub has_log: bool,
}

impl CheckRun {
    pub fn duration_secs(&self, now: DateTime<Utc>) -> Option<i64> {
        let start = self.started_at?;
        Some(
            (self.finished_at.unwrap_or(now) - start)
                .num_seconds()
                .max(0),
        )
    }
}

/// Checks of a PR detail and how to re-run the failed ones.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecksInfo {
    pub runs: Vec<CheckRun>,
    /// GitLab pipeline id or GitHub workflow run ids (comma separated) with failures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerun_target: Option<String>,
    pub can_rerun: bool,
}

impl ChecksInfo {
    /// Runs sorted failing, running, pending, passed, skipped; by name within a group.
    pub fn sorted(&self) -> Vec<&CheckRun> {
        let mut v: Vec<&CheckRun> = self.runs.iter().collect();
        v.sort_by(|a, b| {
            a.status
                .rank()
                .cmp(&b.status.rank())
                .then_with(|| a.name.cmp(&b.name))
        });
        v
    }

    pub fn count(&self, s: CheckStatus) -> usize {
        self.runs.iter().filter(|r| r.status == s).count()
    }
}

/// `4m 12s`, `38s`, `1h 3m`.
pub fn format_duration(secs: i64) -> String {
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m {}s", s / 60, s % 60),
        s => format!("{}h {}m", s / 3600, (s % 3600) / 60),
    }
}

/// `2m ago`, `3h ago`, `just now`.
pub fn ago(t: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let s = (now - t).num_seconds().max(0);
    match s {
        s if s < 45 => "just now".into(),
        s if s < 3600 => format!("{}m ago", (s / 60).max(1)),
        s if s < 86400 => format!("{}h ago", s / 3600),
        s => format!("{}d ago", s / 86400),
    }
}

/// One plain-language line about the checks, for the card header.
pub fn checks_sentence(info: &ChecksInfo, now: DateTime<Utc>) -> String {
    let runs: Vec<&CheckRun> = info
        .runs
        .iter()
        .filter(|r| r.status != CheckStatus::Skipped)
        .collect();
    if runs.is_empty() {
        return "No checks".into();
    }
    let total = runs.len();
    let failed: Vec<&&CheckRun> = runs
        .iter()
        .filter(|r| r.status == CheckStatus::Failed)
        .collect();
    let active = runs
        .iter()
        .filter(|r| matches!(r.status, CheckStatus::Running | CheckStatus::Pending))
        .count();
    if !failed.is_empty() {
        let first = failed[0];
        let when = first
            .finished_at
            .map(|t| format!(" ({})", ago(t, now)))
            .unwrap_or_default();
        let more = if failed.len() > 1 {
            format!(" and {} more", failed.len() - 1)
        } else {
            String::new()
        };
        let running = if active > 0 {
            format!(", {active} running")
        } else {
            String::new()
        };
        return format!(
            "{} failing: {}{when}{more}{running}",
            failed.len(),
            first.name
        );
    }
    if active > 0 {
        // Estimate from the longest finished check against the oldest running one.
        let longest = runs
            .iter()
            .filter_map(|r| r.finished_at.and(r.duration_secs(now)))
            .max();
        let elapsed = runs
            .iter()
            .filter(|r| r.status == CheckStatus::Running)
            .filter_map(|r| r.duration_secs(now))
            .max();
        let left = match (longest, elapsed) {
            (Some(l), Some(e)) if l > e => format!(
                ", ~{} left",
                format_duration(((l - e) / 60).max(1) * 60).trim_end_matches(" 0s")
            ),
            _ => String::new(),
        };
        return format!("Running {active} of {total}{left}");
    }
    let start = runs.iter().filter_map(|r| r.started_at).min();
    let end = runs.iter().filter_map(|r| r.finished_at).max();
    let took = match (start, end) {
        (Some(s), Some(e)) if e > s => format!(" in {}", format_duration((e - s).num_seconds())),
        _ => String::new(),
    };
    let all = if total == 1 {
        "The check passed".to_string()
    } else {
        format!("All {total} checks passed")
    };
    format!("{all}{took}")
}

/// Last `n` lines of a CI log without ANSI colors and GitLab section markers.
pub fn log_tail(log: &str, n: usize) -> Vec<String> {
    let clean: Vec<String> = log
        .lines()
        .map(strip_ansi)
        .map(|l| strip_stamp(l.rsplit('\r').next().unwrap_or_default()).to_string())
        .filter(|l| {
            !l.starts_with("section_start:")
                && !l.starts_with("section_end:")
                && !l.trim().is_empty()
                && !is_runner_noise(l)
        })
        .collect();
    clean[clean.len().saturating_sub(n)..].to_vec()
}

/// GitLab runner prefixes: `2026-10-29T15:38:50.553443Z 01O+`; GitHub Actions: `2026-...Z `.
fn strip_stamp(l: &str) -> &str {
    let b = l.as_bytes();
    if b.len() < 21 || !b[..4].iter().all(u8::is_ascii_digit) || b[4] != b'-' || b[10] != b'T' {
        return l;
    }
    let Some(z) = l.find("Z ") else { return l };
    let rest = &l[z + 2..];
    let tag = rest.as_bytes();
    if tag.len() >= 3 && tag[..2].iter().all(u8::is_ascii_hexdigit) && matches!(tag[2], b'O' | b'E')
    {
        let skip = if tag.get(3) == Some(&b'+') { 4 } else { 3 };
        return rest[skip..].trim_start_matches(' ');
    }
    rest
}

/// Runner housekeeping after the script (artifacts, cache, cleanup).
fn is_runner_noise(l: &str) -> bool {
    let t = l.trim_start();
    [
        "Uploading artifacts",
        "Cleaning up project directory",
        "Saving cache",
        "Created cache",
        "Running after_script",
        "Not uploading cache",
    ]
    .iter()
    .any(|p| t.starts_with(p))
        || t.contains("matching artifact files")
        || t.contains("correlation_id=")
}

fn strip_ansi(l: &str) -> String {
    let mut out = String::with_capacity(l.len());
    let mut chars = l.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for x in chars.by_ref() {
                    if x.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    // GitLab embeds section markers mid-line after a color reset.
    match out.find("section_") {
        Some(i)
            if out[i..].starts_with("section_start:") || out[i..].starts_with("section_end:") =>
        {
            out[..i].to_string()
        }
        _ => out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn run(
        name: &str,
        status: CheckStatus,
        start: i64,
        end: Option<i64>,
        now: DateTime<Utc>,
    ) -> CheckRun {
        CheckRun {
            id: name.into(),
            name: name.into(),
            stage: None,
            status,
            started_at: Some(now - Duration::seconds(start)),
            finished_at: end.map(|e| now - Duration::seconds(e)),
            url: None,
            has_log: false,
        }
    }

    #[test]
    fn sentences() {
        let now = Utc::now();
        let mut i = ChecksInfo::default();
        assert_eq!(checks_sentence(&i, now), "No checks");
        i.runs = vec![
            run("a", CheckStatus::Passed, 300, Some(48), now),
            run("b", CheckStatus::Passed, 200, Some(100), now),
        ];
        assert_eq!(checks_sentence(&i, now), "All 2 checks passed in 4m 12s");
        i.runs
            .push(run("lint", CheckStatus::Failed, 400, Some(120), now));
        assert_eq!(checks_sentence(&i, now), "1 failing: lint (2m ago)");
        i.runs.pop();
        i.runs.push(run("c", CheckStatus::Running, 60, None, now));
        assert_eq!(checks_sentence(&i, now), "Running 1 of 3, ~3m left");
        assert_eq!(i.sorted()[0].name, "c");
    }

    #[test]
    fn tails_logs() {
        let log = "\u{1b}[0Ksection_start:1:step\r\u{1b}[0Kstep\nline 1\n\u{1b}[31;1mERROR\u{1b}[0m boom\nsection_end:1:step\n";
        assert_eq!(log_tail(log, 2), vec!["line 1", "ERROR boom"]);
        let gl = "2026-10-29T15:38:50.553443Z 01O === 1 failed ===\n2026-10-29T15:38:54.116926Z 00O+Uploading artifacts for failed job\n2026-10-29T15:38:56.562034Z 00O ERROR: Job failed\n";
        assert_eq!(
            log_tail(gl, 5),
            vec!["=== 1 failed ===", "ERROR: Job failed"]
        );
        assert_eq!(format_duration(3720), "1h 2m");
    }
}
