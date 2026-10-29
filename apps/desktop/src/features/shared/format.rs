//! Plain-text formatting helpers.

use chrono::{DateTime, Utc};

pub fn relative_time(t: DateTime<Utc>) -> String {
    let s = (Utc::now() - t).num_seconds().max(0);
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", s / 60),
        3600..=86399 => format!("{}h ago", s / 3600),
        86400..=2_591_999 => format!("{}d ago", s / 86400),
        _ => format!("{}mo ago", s / 2_592_000),
    }
}
