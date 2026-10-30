//! Suggested changes in comment bodies (GitHub ```suggestion, GitLab ```suggestion:-N+M) and code
//! context around an inline thread's anchor, read from the cached diff.

use crate::{ChangedFile, DiffLine, DiffPosition, DiffSide};

/// A suggestion block: replaces the anchored line(s) widened by `above` / `below` lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub above: u32,
    pub below: u32,
    pub lines: Vec<String>,
}

/// A comment body split around suggestion blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyPart {
    Text(String),
    Suggestion(Suggestion),
}

/// `suggestion`, `suggestion:-2+1` -> (above, below); None for other info strings.
fn suggestion_info(info: &str) -> Option<(u32, u32)> {
    let rest = info.trim().strip_prefix("suggestion")?;
    if rest.is_empty() {
        return Some((0, 0));
    }
    let rest = rest.strip_prefix(":-")?;
    let (a, b) = rest.split_once('+')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

/// Split `body` into text and suggestion parts; an unclosed suggestion fence stays text.
pub fn split_suggestions(body: &str) -> Vec<BodyPart> {
    let mut parts = vec![];
    let mut text = String::new();
    let lines: Vec<&str> = body.split('\n').collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim_start();
        let ticks = t.chars().take_while(|&c| c == '`').count();
        let info = (ticks >= 3 && line.len() - t.len() <= 3)
            .then(|| suggestion_info(&t[ticks..]))
            .flatten();
        let close = info.and_then(|_| {
            (i + 1..lines.len()).find(|&j| {
                let c = lines[j].trim();
                c.len() >= ticks && c.chars().all(|ch| ch == '`')
            })
        });
        match (info, close) {
            (Some((above, below)), Some(end)) => {
                if !text.trim().is_empty() {
                    parts.push(BodyPart::Text(std::mem::take(&mut text)));
                }
                text.clear();
                let lines = lines[i + 1..end]
                    .iter()
                    .map(|l| l.trim_end_matches('\r').to_string())
                    .collect();
                parts.push(BodyPart::Suggestion(Suggestion {
                    above,
                    below,
                    lines,
                }));
                i = end + 1;
            }
            _ => {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(line);
                i += 1;
            }
        }
    }
    if !text.trim().is_empty() {
        parts.push(BodyPart::Text(text));
    }
    parts
}

/// The plain suggestion text (for "Copy suggestion").
pub fn suggestion_text(s: &Suggestion) -> String {
    s.lines.join("\n")
}

fn line_no(l: &DiffLine, side: DiffSide) -> Option<u32> {
    match side {
        DiffSide::New => l.new_no,
        DiffSide::Old => l.old_no,
    }
}

/// Diff lines of the hunk holding `pos.line`, from `context` lines before the range start
/// through the anchor; at most `max` lines, keeping the anchor.
pub fn code_context(
    file: &ChangedFile,
    pos: &DiffPosition,
    context: usize,
    max: usize,
) -> Vec<DiffLine> {
    let start = pos.start_line.unwrap_or(pos.line).min(pos.line);
    for h in &file.hunks {
        let Some(end) = h
            .lines
            .iter()
            .position(|l| line_no(l, pos.side) == Some(pos.line))
        else {
            continue;
        };
        let first = h.lines[..=end]
            .iter()
            .position(|l| line_no(l, pos.side).is_some_and(|n| n >= start))
            .unwrap_or(end);
        let from = first
            .saturating_sub(context)
            .max((end + 1).saturating_sub(max));
        return h.lines[from..=end].to_vec();
    }
    vec![]
}

/// Lines `from..=to` of the given side, as text, when the diff shows all of them.
pub fn side_lines(file: &ChangedFile, side: DiffSide, from: u32, to: u32) -> Option<Vec<String>> {
    (from..=to)
        .map(|n| {
            file.hunks
                .iter()
                .flat_map(|h| &h.lines)
                .find(|l| line_no(l, side) == Some(n))
                .map(|l| l.text.clone())
        })
        .collect()
}

/// Original lines a suggestion replaces: the anchored range widened by the GitLab offsets.
pub fn suggestion_original(
    file: &ChangedFile,
    pos: &DiffPosition,
    s: &Suggestion,
) -> Option<(u32, Vec<String>)> {
    let start = pos.start_line.unwrap_or(pos.line).min(pos.line);
    let from = start.checked_sub(s.above).filter(|&n| n > 0)?;
    let to = pos.line + s.below;
    Some((from, side_lines(file, DiffSide::New, from, to)?))
}

#[cfg(test)]
#[path = "suggestion_tests.rs"]
mod tests;
