//! Per-OS differences in one place: key modifiers and their labels, log folder, reduced motion.
//! Key strings everywhere are written macOS style (`cmd-k`); `key` maps them for the running OS.

use std::path::PathBuf;

pub const MAC: bool = cfg!(target_os = "macos");

/// A binding written with `cmd` as this OS expects it: unchanged on macOS, `cmd` as `ctrl` elsewhere.
pub fn key(k: &str) -> String {
    if MAC {
        k.to_string()
    } else {
        to_ctrl(k)
    }
}

pub(crate) fn to_ctrl(k: &str) -> String {
    // macOS full screen (Ctrl+Cmd+F) is F11 elsewhere.
    if k == "ctrl-cmd-f" {
        return "f11".into();
    }
    let mut mods: Vec<&str> = Vec::new();
    let mut rest = k;
    // Modifiers are `name-` prefixes; the key itself may be `-` (`cmd--`).
    while let Some((m, r)) = rest.split_once('-') {
        if m.is_empty() || r.is_empty() {
            break;
        }
        mods.push(m);
        rest = r;
    }
    // A macOS Ctrl chord (`ctrl-r`) would collide with the Cmd one (`cmd-r`): add Shift.
    let plain_ctrl = mods.contains(&"ctrl") && !mods.contains(&"cmd") && !mods.contains(&"shift");
    let mut out: Vec<&str> = Vec::new();
    for m in mods {
        let m = if m == "cmd" { "ctrl" } else { m };
        if !out.contains(&m) {
            out.push(m);
            if m == "ctrl" && plain_ctrl {
                out.push("shift");
            }
        }
    }
    out.push(rest);
    out.join("-")
}

/// Label of one key or modifier as shown on keycaps.
pub fn key_label(k: &str) -> String {
    let s = match (MAC, k) {
        (true, "cmd") => "\u{2318}",
        (true, "shift") => "\u{21e7}",
        (true, "alt") => "\u{2325}",
        (true, "ctrl") => "\u{2303}",
        (true, "enter") => "\u{21a9}",
        (false, "ctrl" | "cmd") => "Ctrl",
        (false, "shift") => "Shift",
        (false, "alt") => "Alt",
        (false, "enter") => "Enter",
        (_, "up") => "\u{2191}",
        (_, "down") => "\u{2193}",
        (_, "left") => "\u{2190}",
        (_, "right") => "\u{2192}",
        (_, "escape") => "esc",
        (_, "") => "-",
        _ => return k.to_uppercase(),
    };
    s.to_string()
}

/// Separator between keycap labels: `⌘ R` on macOS, `Ctrl+R` elsewhere.
pub const KEY_SEP: &str = if MAC { " " } else { "+" };

/// Crash and diagnostic logs: `~/Library/Logs/Confluo`, else `$XDG_STATE_HOME/confluo` (`~/.local/state`).
pub fn log_dir() -> PathBuf {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    if MAC {
        return home.join("Library/Logs/Confluo");
    }
    std::env::var_os("XDG_STATE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"))
        .join("confluo")
}

/// The OS asks for reduced motion. macOS: Accessibility setting. Elsewhere: GNOME's
/// `enable-animations` off, or `CONFLUO_REDUCE_MOTION=1`. Off when unknown.
pub fn prefers_reduced_motion() -> bool {
    if std::env::var("CONFLUO_REDUCE_MOTION").is_ok_and(|v| v == "1") {
        return true;
    }
    let (cmd, args): (&str, &[&str]) = if MAC {
        (
            "defaults",
            &["read", "com.apple.universalaccess", "reduceMotion"],
        )
    } else {
        (
            "gsettings",
            &["get", "org.gnome.desktop.interface", "enable-animations"],
        )
    };
    let Ok(out) = std::process::Command::new(cmd).args(args).output() else {
        return false;
    };
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if MAC {
        v == "1"
    } else {
        v == "false"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_becomes_ctrl() {
        assert_eq!(to_ctrl("cmd-k"), "ctrl-k");
        assert_eq!(to_ctrl("cmd-shift-l"), "ctrl-shift-l");
        assert_eq!(to_ctrl("alt-cmd-down"), "alt-ctrl-down");
        assert_eq!(to_ctrl("cmd--"), "ctrl--");
        assert_eq!(to_ctrl("ctrl-cmd-f"), "f11");
        assert_eq!(to_ctrl("ctrl-cmd-x"), "ctrl-x");
        assert_eq!(to_ctrl("ctrl-r"), "ctrl-shift-r");
        assert_eq!(to_ctrl("ctrl-p"), "ctrl-shift-p");
        assert_eq!(to_ctrl("/"), "/");
        assert_eq!(to_ctrl("-"), "-");
    }
}
