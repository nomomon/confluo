//! Crash logs: a panic hook writes `~/Library/Logs/Confluo/crash-<timestamp>.log` (message, location,
//! backtrace) and keeps the last `KEEP`. GPUI runs app code inside platform callbacks that cannot
//! unwind, so a panic aborts the process; the log is the only trace. The next launch reports a log
//! it has not seen yet (`last-seen` marker) with a quiet toast.

use std::path::{Path, PathBuf};

const KEEP: usize = 10;
const SEEN: &str = "last-seen";

pub fn log_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Logs/Confluo")
}

pub fn install() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(non-string panic payload)".into());
        let loc = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_default();
        let thread = std::thread::current()
            .name()
            .unwrap_or("<unnamed>")
            .to_string();
        let now = chrono::Local::now();
        let text = format!(
            "Confluo {} crashed at {}\nthread '{thread}' panicked at {loc}:\n{msg}\n\nbacktrace:\n{bt}\n",
            env!("CARGO_PKG_VERSION"),
            now.to_rfc3339()
        );
        let dir = log_dir();
        if std::fs::create_dir_all(&dir).is_ok() {
            let path = dir.join(format!("crash-{}.log", now.format("%Y%m%d-%H%M%S%.3f")));
            let _ = std::fs::write(&path, &text);
            prune(&dir, KEEP);
        }
        prev(info);
    }));
}

fn crash_logs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("crash-") && n.ends_with(".log"))
        })
        .collect();
    // Timestamped names sort chronologically.
    v.sort();
    v
}

fn prune(dir: &Path, keep: usize) {
    let logs = crash_logs(dir);
    for p in logs.iter().take(logs.len().saturating_sub(keep)) {
        let _ = std::fs::remove_file(p);
    }
}

/// The newest crash log not reported yet; marks it as seen.
pub fn take_unseen() -> Option<PathBuf> {
    take_unseen_in(&log_dir())
}

fn take_unseen_in(dir: &Path) -> Option<PathBuf> {
    let newest = crash_logs(dir).pop()?;
    let name = newest.file_name()?.to_str()?.to_string();
    let seen = std::fs::read_to_string(dir.join(SEEN)).unwrap_or_default();
    if seen.trim() >= name.as_str() {
        return None;
    }
    let _ = std::fs::write(dir.join(SEEN), &name);
    Some(newest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prunes_and_reports_each_log_once() {
        let dir = std::env::temp_dir().join(format!("confluo-crash-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..12 {
            std::fs::write(dir.join(format!("crash-20260101-0000{i:02}.000.log")), "x").unwrap();
        }
        prune(&dir, KEEP);
        assert_eq!(crash_logs(&dir).len(), KEEP);
        let first = take_unseen_in(&dir).unwrap();
        assert!(first.ends_with("crash-20260101-000011.000.log"));
        assert_eq!(take_unseen_in(&dir), None);
        std::fs::write(dir.join("crash-20260102-000000.000.log"), "x").unwrap();
        assert!(take_unseen_in(&dir).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
