//! Short, readable failure reasons for toasts ("GitLab: not found (404)"), never raw URLs or bodies.

use crate::ProviderError;

/// Product name for a host or URL ("GitLab", "GitHub", "Azure DevOps"), else the host itself.
fn product(host_or_url: &str) -> String {
    let h = host_or_url
        .split("://")
        .nth(1)
        .unwrap_or(host_or_url)
        .split('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if h.contains("gitlab") {
        "GitLab".into()
    } else if h.contains("github") {
        "GitHub".into()
    } else if h.contains("dev.azure") || h.contains("visualstudio") {
        "Azure DevOps".into()
    } else {
        h
    }
}

/// The JSON `message` (GitLab, GitHub) when short and on one line.
fn message(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let m = v.get("message")?;
    let m = m
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| m.to_string());
    (m.len() <= 80 && !m.contains('\n')).then_some(m)
}

impl ProviderError {
    /// One short line for the UI.
    pub fn reason(&self) -> String {
        match self {
            Self::Unauthenticated(h) => format!("{}: not signed in (401)", product(h)),
            Self::NotFound(url) => format!("{}: not found (404)", product(url)),
            Self::Http { status, body } => match message(body) {
                Some(m) => format!("HTTP {status}: {m}"),
                None => format!("HTTP {status}"),
            },
            Self::Network(_) => "network error".into(),
            Self::RateLimited { host, until } => format!(
                "{}: rate limited until {}",
                product(host),
                until.with_timezone(&chrono::Local).format("%H:%M")
            ),
            Self::Unsupported(w) => format!("not supported: {w}"),
            Self::Decode(_) => "unexpected response".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_are_short() {
        let e = ProviderError::NotFound("https://gitlab.com/api/v4/projects/a%2Fb/notes/1".into());
        assert_eq!(e.reason(), "GitLab: not found (404)");
        let e = ProviderError::Http {
            status: 403,
            body: r#"{"message":"403 Forbidden"}"#.into(),
        };
        assert_eq!(e.reason(), "HTTP 403: 403 Forbidden");
        assert_eq!(
            ProviderError::Unauthenticated("github.com".into()).reason(),
            "GitHub: not signed in (401)"
        );
    }
}
