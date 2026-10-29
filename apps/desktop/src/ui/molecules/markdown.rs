//! Very small markdown renderer: headings, bullets, code fences, quotes, paragraphs.

use gpui::{div, prelude::*, px, AnyElement, FontWeight};

use crate::ui::theme;

pub fn markdown(src: &str) -> Vec<AnyElement> {
    let mut out = vec![];
    let mut in_code = false;
    let mut code = String::new();
    for line in src.lines() {
        let t = line.trim_end();
        if t.trim_start().starts_with("```") {
            if in_code {
                out.push(
                    div()
                        .p_2()
                        .rounded_md()
                        .bg(theme::field())
                        .text_xs()
                        .font_family(theme::MONO_FONT)
                        .child(std::mem::take(&mut code))
                        .into_any_element(),
                );
            }
            in_code = !in_code;
            continue;
        }
        if in_code {
            if !code.is_empty() {
                code.push('\n');
            }
            code.push_str(t);
            continue;
        }
        let trimmed = t.trim_start();
        let el = if trimmed.is_empty() {
            div().h(px(4.))
        } else if let Some(h) = trimmed.strip_prefix("### ").or_else(|| trimmed.strip_prefix("#### ")) {
            div().pt_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(strip_inline(h))
        } else if let Some(h) = trimmed.strip_prefix("## ") {
            div().pt_1().text_base().font_weight(FontWeight::SEMIBOLD).child(strip_inline(h))
        } else if let Some(h) = trimmed.strip_prefix("# ") {
            div().pt_1().text_lg().font_weight(FontWeight::SEMIBOLD).child(strip_inline(h))
        } else if let Some(b) = trimmed.strip_prefix("- [ ] ").or_else(|| trimmed.strip_prefix("* [ ] ")) {
            div().flex().gap_2().child("\u{2610}").child(strip_inline(b))
        } else if let Some(b) = trimmed.strip_prefix("- [x] ").or_else(|| trimmed.strip_prefix("* [x] ")) {
            div().flex().gap_2().child("\u{2611}").child(strip_inline(b))
        } else if let Some(b) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            let indent = (t.len() - trimmed.len()) as f32 * 6.;
            div().flex().gap_2().pl(px(indent)).child(div().text_color(theme::muted()).child("\u{2022}")).child(strip_inline(b))
        } else if let Some(q) = trimmed.strip_prefix("> ") {
            div().pl_2().border_l_2().border_color(theme::border()).text_color(theme::muted()).child(strip_inline(q))
        } else {
            div().child(strip_inline(trimmed))
        };
        out.push(el.text_sm().into_any_element());
    }
    if in_code && !code.is_empty() {
        out.push(div().p_2().rounded_md().bg(theme::field()).text_xs().font_family(theme::MONO_FONT).child(code).into_any_element());
    }
    out
}

/// Drop the most common inline markers so text reads cleanly.
fn strip_inline(s: &str) -> String {
    let mut s = s.replace("**", "").replace("__", "").replace('`', "");
    // [text](url) -> text (url)
    while let Some(start) = s.find('[') {
        let Some(mid) = s[start..].find("](").map(|i| start + i) else { break };
        let Some(end) = s[mid..].find(')').map(|i| mid + i) else { break };
        let text = s[start + 1..mid].to_string();
        let url = s[mid + 2..end].to_string();
        s.replace_range(start..=end, &format!("{text} ({url})"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::strip_inline;

    #[test]
    fn strips_inline_markers() {
        assert_eq!(strip_inline("see [docs](http://x) **now**"), "see docs (http://x) now");
    }
}
