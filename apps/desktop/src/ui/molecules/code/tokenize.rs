//! Line-at-a-time tokenizer: keywords, strings, comments, numbers, types and keys.
//! No cross-line state, so block comments and multi-line strings color only their own line.

use std::ops::Range;

use super::lang::Lang;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tok {
    Keyword,
    String,
    Comment,
    Number,
    Type,
}

/// Highlighted byte ranges of `line`, in order and non-overlapping.
pub fn tokenize(line: &str, lang: Lang) -> Vec<(Range<usize>, Tok)> {
    let mut out = vec![];
    match lang {
        Lang::Plain => return out,
        Lang::Markdown => {
            let t = line.trim_start();
            if t.starts_with('#') || t.starts_with("```") {
                out.push((0..line.len(), Tok::Keyword));
            }
            return out;
        }
        Lang::Docker => {
            let word = line.split_whitespace().next().unwrap_or("");
            if !word.is_empty() && word.chars().all(|c| c.is_ascii_uppercase()) {
                let start = line.len() - line.trim_start().len();
                out.push((start..start + word.len(), Tok::Keyword));
            }
        }
        Lang::Yaml | Lang::Toml => key_prefix(line, lang, &mut out),
        _ => {}
    }
    let b = line.as_bytes();
    let mut i = out.last().map(|(r, _)| r.end).unwrap_or(0);
    while i < b.len() {
        let c = b[i];
        if lang
            .line_comment()
            .is_some_and(|lc| line[i..].starts_with(lc))
        {
            out.push((i..b.len(), Tok::Comment));
            break;
        }
        if line[i..].starts_with("/*") && matches!(lang, Lang::Rust | Lang::Ts | Lang::Go) {
            let end = line[i + 2..]
                .find("*/")
                .map(|e| i + e + 4)
                .unwrap_or(b.len());
            out.push((i..end, Tok::Comment));
            i = end;
        } else if c == b'"' || c == b'\'' || c == b'`' {
            if c == b'\'' && lang == Lang::Rust && is_lifetime(b, i) {
                i += 1;
                continue;
            }
            let mut j = i + 1;
            while j < b.len() && b[j] != c {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            let end = (j + 1).min(b.len());
            out.push((i..end, Tok::String));
            i = end;
        } else if c.is_ascii_digit() && (i == 0 || !is_ident(b[i - 1])) {
            let mut j = i;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'.' || b[j] == b'_') {
                j += 1;
            }
            out.push((i..j, Tok::Number));
            i = j;
        } else if is_ident(c) && !c.is_ascii_digit() {
            let mut j = i;
            while j < b.len() && is_ident(b[j]) {
                j += 1;
            }
            let w = &line[i..j];
            if lang.keywords().contains(&w) {
                out.push((i..j, Tok::Keyword));
            } else if lang.caps_are_types() && w.as_bytes()[0].is_ascii_uppercase() {
                out.push((i..j, Tok::Type));
            }
            i = j;
        } else {
            i += 1;
            while i < b.len() && !line.is_char_boundary(i) {
                i += 1;
            }
        }
    }
    out
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

/// `'a` without a closing quote two chars later.
fn is_lifetime(b: &[u8], i: usize) -> bool {
    b.get(i + 2) != Some(&b'\'')
        && b.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic())
        && b.get(i + 3) != Some(&b'\'')
}

/// YAML `key:` and TOML `key =` / `[section]` prefixes.
fn key_prefix(line: &str, lang: Lang, out: &mut Vec<(Range<usize>, Tok)>) {
    let start = line.len() - line.trim_start().len();
    let rest = &line[start..];
    if lang == Lang::Toml && rest.starts_with('[') {
        out.push((start..line.len(), Tok::Keyword));
        return;
    }
    let rest_start = start + if rest.starts_with("- ") { 2 } else { 0 };
    let sep = if lang == Lang::Yaml { ':' } else { '=' };
    if let Some(k) = line[rest_start..].find(sep) {
        let key = line[rest_start..rest_start + k].trim_end();
        if !key.is_empty() && !key.contains(['"', '\'', '#', ' ', '{', '(']) {
            out.push((rest_start..rest_start + key.len(), Tok::Keyword));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str, lang: Lang) -> Vec<(&str, Tok)> {
        tokenize(line, lang)
            .into_iter()
            .map(|(r, t)| (&line[r], t))
            .collect()
    }

    #[test]
    fn rust_line() {
        let k = kinds("pub fn x(a: &'a str) -> Foo { \"s\" } // c", Lang::Rust);
        assert_eq!(
            k,
            vec![
                ("pub", Tok::Keyword),
                ("fn", Tok::Keyword),
                ("Foo", Tok::Type),
                ("\"s\"", Tok::String),
                ("// c", Tok::Comment)
            ]
        );
    }

    #[test]
    fn yaml_key_and_comment() {
        let k = kinds("  args: 3 # note", Lang::Yaml);
        assert_eq!(
            k,
            vec![
                ("args", Tok::Keyword),
                ("3", Tok::Number),
                ("# note", Tok::Comment)
            ]
        );
    }

    #[test]
    fn plain_and_unicode_are_safe() {
        assert!(tokenize("anything", Lang::Plain).is_empty());
        let k = kinds("let é = \"ü\";", Lang::Rust);
        assert_eq!(k[1], ("\"ü\"", Tok::String));
    }

    #[test]
    fn detects_languages() {
        assert_eq!(Lang::from_path("a/Dockerfile"), Lang::Docker);
        assert_eq!(Lang::from_path("x.tsx"), Lang::Ts);
        assert_eq!(Lang::from_path("Makefile"), Lang::Plain);
    }
}
