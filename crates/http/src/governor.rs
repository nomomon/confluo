//! Per-host request governor: token bucket, concurrency cap, and pauses from rate limit signals.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Steady request rate per host.
pub const RATE_PER_SEC: f64 = 5.0;
/// Requests allowed in a burst before the steady rate applies.
pub const BURST: f64 = 10.0;
/// Requests in flight per host.
pub const MAX_CONCURRENT: usize = 4;
/// Rate used while the host reports a low remaining budget.
pub const SLOW_RATE_PER_SEC: f64 = 0.5;
/// Pauses shorter than this are waited out; longer ones fail fast with `RateLimited`.
pub const MAX_INLINE_WAIT: Duration = Duration::from_secs(10);
/// First backoff after a 429 / secondary limit without a server hint.
pub const BASE_BACKOFF: Duration = Duration::from_secs(60);
pub const MAX_BACKOFF: Duration = Duration::from_secs(15 * 60);

/// Token bucket with an injectable clock, so it is testable.
#[derive(Debug, Clone)]
pub struct Bucket {
    tokens: f64,
    last: Instant,
    pub rate: f64,
    pub burst: f64,
}

impl Bucket {
    pub fn new(rate: f64, burst: f64, now: Instant) -> Self {
        Self {
            tokens: burst,
            last: now,
            rate,
            burst,
        }
    }

    /// Take one token. Returns how long to wait before the request may go (the token is reserved).
    pub fn take(&mut self, now: Instant) -> Duration {
        let dt = now.saturating_duration_since(self.last).as_secs_f64();
        self.tokens = (self.tokens + dt * self.rate).min(self.burst);
        self.last = now;
        self.tokens -= 1.0;
        if self.tokens >= 0.0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(-self.tokens / self.rate)
        }
    }
}

/// Backoff for the n-th consecutive limit hit (0-based), with `jitter` in 0..1 adding up to 20%.
pub fn backoff(failures: u32, jitter: f64) -> Duration {
    let base = BASE_BACKOFF.as_secs_f64() * 2f64.powi(failures.min(8) as i32);
    let capped = base.min(MAX_BACKOFF.as_secs_f64());
    Duration::from_secs_f64(capped * (1.0 + 0.2 * jitter.clamp(0.0, 1.0)))
}

/// Cheap jitter in 0..1 without a rand dependency.
pub fn jitter() -> f64 {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    f64::from(n % 1000) / 1000.0
}

/// What a response says about the host's rate limit.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    /// 429, or a GitHub secondary-limit 403.
    pub limited: bool,
    /// `Retry-After` in seconds.
    pub retry_after: Option<u64>,
    pub remaining: Option<u64>,
    pub limit: Option<u64>,
    /// Unix seconds when the budget resets.
    pub reset: Option<i64>,
}

/// Read rate limit headers (GitHub `x-ratelimit-*`, GitLab `ratelimit-*`, `retry-after`).
/// `body` is consulted only for a 403 without headers that settle it.
pub fn read_signal(status: u16, header: impl Fn(&str) -> Option<String>, body: &[u8]) -> Signal {
    let num = |names: &[&str]| {
        names
            .iter()
            .find_map(|n| header(n))
            .and_then(|v| v.trim().parse::<i64>().ok())
    };
    let remaining = num(&["x-ratelimit-remaining", "ratelimit-remaining"]).map(|v| v.max(0) as u64);
    let limit = num(&["x-ratelimit-limit", "ratelimit-limit"]).map(|v| v.max(0) as u64);
    let reset = num(&["x-ratelimit-reset", "ratelimit-reset"]);
    let retry_after = num(&["retry-after"]).map(|v| v.max(0) as u64);
    let limited = match status {
        429 => true,
        403 => {
            retry_after.is_some() || remaining == Some(0) || {
                let b = String::from_utf8_lossy(&body[..body.len().min(2048)]).to_ascii_lowercase();
                b.contains("rate limit")
            }
        }
        _ => false,
    };
    Signal {
        limited,
        retry_after,
        remaining,
        limit,
        reset,
    }
}

#[derive(Debug)]
struct State {
    bucket: Bucket,
    /// Nothing is sent before this.
    paused_until: Option<(Instant, DateTime<Utc>)>,
    failures: u32,
}

/// Shared limiter for one host. Get it with [`governor`].
#[derive(Debug)]
pub struct Governor {
    pub host: String,
    state: Mutex<State>,
    slots: Arc<Semaphore>,
}

/// Outcome of applying a response to the governor.
#[derive(Debug, PartialEq)]
pub enum Verdict {
    Ok,
    /// Limited; the host is paused for this long.
    Limited(Duration),
}

impl Governor {
    pub fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            state: Mutex::new(State {
                bucket: Bucket::new(RATE_PER_SEC, BURST, Instant::now()),
                paused_until: None,
                failures: 0,
            }),
            slots: Arc::new(Semaphore::new(MAX_CONCURRENT)),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// When the host is paused, the wall-clock end of the pause.
    pub fn paused_until(&self) -> Option<DateTime<Utc>> {
        let s = self.lock();
        s.paused_until
            .filter(|(i, _)| *i > Instant::now())
            .map(|(_, w)| w)
    }

    /// Wait for a slot and a token. Fails fast when the host is paused for longer than `MAX_INLINE_WAIT`.
    pub async fn acquire(&self) -> Result<OwnedSemaphorePermit, DateTime<Utc>> {
        let pause = {
            let s = self.lock();
            s.paused_until
                .map(|(i, w)| (i.saturating_duration_since(Instant::now()), w))
        };
        if let Some((left, wall)) = pause {
            if left > MAX_INLINE_WAIT {
                return Err(wall);
            }
            if !left.is_zero() {
                tokio::time::sleep(left).await;
            }
        }
        let permit = self
            .slots
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore never closed");
        let wait = self.lock().bucket.take(Instant::now());
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
        Ok(permit)
    }

    /// Apply a response's signal: slow down when the budget runs low, pause on limits.
    pub fn observe(&self, sig: &Signal) -> Verdict {
        let now = Instant::now();
        let now_wall = Utc::now();
        let mut s = self.lock();
        let until_reset = sig
            .reset
            .map(|r| Duration::from_secs((r - now_wall.timestamp()).max(1) as u64));
        if sig.limited {
            let wait = match (sig.retry_after, until_reset) {
                (Some(ra), _) => Duration::from_secs(ra.max(1)),
                (None, Some(r)) if sig.remaining == Some(0) => r,
                _ => backoff(s.failures, jitter()),
            };
            s.failures += 1;
            s.paused_until = Some((
                now + wait,
                now_wall + chrono::Duration::from_std(wait).unwrap_or_default(),
            ));
            return Verdict::Limited(wait);
        }
        s.failures = 0;
        if let (Some(0), Some(r)) = (sig.remaining, until_reset) {
            s.paused_until = Some((
                now + r,
                now_wall + chrono::Duration::from_std(r).unwrap_or_default(),
            ));
        }
        let low = match (sig.remaining, sig.limit) {
            (Some(rem), Some(lim)) if lim > 0 => rem * 10 < lim || rem < 20,
            (Some(rem), None) => rem < 20,
            _ => false,
        };
        let rate = if low { SLOW_RATE_PER_SEC } else { RATE_PER_SEC };
        if (s.bucket.rate - rate).abs() > f64::EPSILON {
            s.bucket.rate = rate;
            s.bucket.burst = if low { 1.0 } else { BURST };
        }
        Verdict::Ok
    }
}

fn registry() -> &'static Mutex<HashMap<String, Arc<Governor>>> {
    static R: OnceLock<Mutex<HashMap<String, Arc<Governor>>>> = OnceLock::new();
    R.get_or_init(Default::default)
}

/// The process-wide governor for a host (lowercased), shared by every client talking to it.
pub fn governor(host: &str) -> Arc<Governor> {
    let host = host.to_ascii_lowercase();
    let mut m = registry().lock().unwrap_or_else(|e| e.into_inner());
    m.entry(host.clone())
        .or_insert_with(|| Arc::new(Governor::new(&host)))
        .clone()
}

/// Latest end of any current pause, across hosts, with the host.
pub fn paused() -> Vec<(String, DateTime<Utc>)> {
    let m = registry().lock().unwrap_or_else(|e| e.into_inner());
    m.values()
        .filter_map(|g| g.paused_until().map(|t| (g.host.clone(), t)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_allows_burst_then_paces() {
        let t0 = Instant::now();
        let mut b = Bucket::new(5.0, 10.0, t0);
        for _ in 0..10 {
            assert_eq!(b.take(t0), Duration::ZERO);
        }
        let w = b.take(t0);
        assert!((w.as_secs_f64() - 0.2).abs() < 1e-6, "{w:?}");
        // After a second, five tokens came back (one was already reserved).
        let mut b2 = Bucket::new(5.0, 10.0, t0);
        for _ in 0..10 {
            b2.take(t0);
        }
        let t1 = t0 + Duration::from_secs(1);
        for _ in 0..5 {
            assert_eq!(b2.take(t1), Duration::ZERO);
        }
        assert!(b2.take(t1) > Duration::ZERO);
    }

    #[test]
    fn bucket_never_exceeds_burst() {
        let t0 = Instant::now();
        let mut b = Bucket::new(5.0, 10.0, t0);
        let later = t0 + Duration::from_secs(3600);
        let free = (0..20).filter(|_| b.take(later).is_zero()).count();
        assert_eq!(free, 10);
    }

    #[test]
    fn backoff_grows_caps_and_jitters() {
        assert_eq!(backoff(0, 0.0), Duration::from_secs(60));
        assert_eq!(backoff(1, 0.0), Duration::from_secs(120));
        assert_eq!(backoff(20, 0.0), MAX_BACKOFF);
        let j = backoff(0, 1.0).as_secs_f64();
        assert!((j - 72.0).abs() < 1e-6);
    }

    fn hdr<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |n| {
            pairs
                .iter()
                .find(|(k, _)| *k == n)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn reads_signals() {
        let s = read_signal(429, hdr(&[("retry-after", "30")]), b"");
        assert!(s.limited);
        assert_eq!(s.retry_after, Some(30));
        let s = read_signal(403, hdr(&[]), b"You have exceeded a secondary rate limit");
        assert!(s.limited);
        let s = read_signal(403, hdr(&[]), b"Resource not accessible by integration");
        assert!(!s.limited);
        let s = read_signal(
            200,
            hdr(&[
                ("ratelimit-remaining", "12"),
                ("ratelimit-limit", "2000"),
                ("ratelimit-reset", "1700000000"),
            ]),
            b"",
        );
        assert_eq!(
            (s.limited, s.remaining, s.limit, s.reset),
            (false, Some(12), Some(2000), Some(1_700_000_000))
        );
        let s = read_signal(403, hdr(&[("x-ratelimit-remaining", "0")]), b"");
        assert!(s.limited);
    }

    #[test]
    fn limit_pauses_with_retry_after_and_ok_clears_failures() {
        let g = Governor::new("example.test");
        let v = g.observe(&Signal {
            limited: true,
            retry_after: Some(120),
            remaining: None,
            limit: None,
            reset: None,
        });
        assert_eq!(v, Verdict::Limited(Duration::from_secs(120)));
        let until = g.paused_until().expect("paused");
        assert!(until > Utc::now() + chrono::Duration::seconds(100));
        // Second hit without hints backs off exponentially from the failure count.
        let v = g.observe(&Signal {
            limited: true,
            retry_after: None,
            remaining: None,
            limit: None,
            reset: None,
        });
        match v {
            Verdict::Limited(d) => {
                assert!(d >= Duration::from_secs(120) && d <= Duration::from_secs(144))
            }
            _ => panic!(),
        }
        g.observe(&Signal {
            limited: false,
            retry_after: None,
            remaining: Some(1000),
            limit: Some(5000),
            reset: None,
        });
        assert_eq!(g.lock().failures, 0);
    }

    #[test]
    fn low_budget_slows_and_zero_pauses_until_reset() {
        let g = Governor::new("example.test");
        g.observe(&Signal {
            limited: false,
            retry_after: None,
            remaining: Some(30),
            limit: Some(5000),
            reset: None,
        });
        assert_eq!(g.lock().bucket.rate, SLOW_RATE_PER_SEC);
        g.observe(&Signal {
            limited: false,
            retry_after: None,
            remaining: Some(4000),
            limit: Some(5000),
            reset: None,
        });
        assert_eq!(g.lock().bucket.rate, RATE_PER_SEC);
        let reset = Utc::now().timestamp() + 600;
        g.observe(&Signal {
            limited: false,
            retry_after: None,
            remaining: Some(0),
            limit: Some(5000),
            reset: Some(reset),
        });
        assert!(g.paused_until().is_some());
    }

    #[tokio::test]
    async fn long_pause_fails_fast() {
        let g = Governor::new("example.test");
        g.observe(&Signal {
            limited: true,
            retry_after: Some(600),
            remaining: None,
            limit: None,
            reset: None,
        });
        let started = Instant::now();
        assert!(g.acquire().await.is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn registry_shares_per_host() {
        assert!(Arc::ptr_eq(
            &governor("GitLab.com"),
            &governor("gitlab.com")
        ));
        assert!(!Arc::ptr_eq(&governor("a.test"), &governor("b.test")));
    }
}
