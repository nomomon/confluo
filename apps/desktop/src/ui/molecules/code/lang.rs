//! Language detection by file name and per-language lexical rules.

/// Languages the lightweight tokenizer knows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Rust,
    Ts,
    Python,
    Go,
    Yaml,
    Json,
    Toml,
    Markdown,
    Shell,
    Docker,
    Plain,
}

impl Lang {
    pub fn from_path(path: &str) -> Lang {
        let name = path.rsplit('/').next().unwrap_or(path);
        if name == "Dockerfile" || name.starts_with("Dockerfile.") || name.ends_with(".dockerfile") {
            return Lang::Docker;
        }
        match name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).as_deref() {
            Some("rs") => Lang::Rust,
            Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs") => Lang::Ts,
            Some("py") => Lang::Python,
            Some("go") => Lang::Go,
            Some("yml" | "yaml") => Lang::Yaml,
            Some("json") => Lang::Json,
            Some("toml") => Lang::Toml,
            Some("md" | "markdown") => Lang::Markdown,
            Some("sh" | "bash" | "zsh") => Lang::Shell,
            _ => Lang::Plain,
        }
    }

    pub(super) fn line_comment(self) -> Option<&'static str> {
        match self {
            Lang::Rust | Lang::Ts | Lang::Go => Some("//"),
            Lang::Python | Lang::Yaml | Lang::Toml | Lang::Shell | Lang::Docker => Some("#"),
            _ => None,
        }
    }

    /// Whether capitalized identifiers read as types.
    pub(super) fn caps_are_types(self) -> bool {
        matches!(self, Lang::Rust | Lang::Ts | Lang::Go | Lang::Python)
    }

    pub(super) fn keywords(self) -> &'static [&'static str] {
        match self {
            Lang::Rust => &[
                "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
                "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true",
                "type", "unsafe", "use", "where", "while",
            ],
            Lang::Ts => &[
                "async", "await", "break", "case", "catch", "class", "const", "continue", "default", "delete", "else", "export", "extends", "false",
                "finally", "for", "from", "function", "if", "import", "in", "instanceof", "interface", "let", "new", "null", "of", "return", "switch",
                "this", "throw", "true", "try", "type", "typeof", "undefined", "var", "void", "while", "yield",
            ],
            Lang::Python => &[
                "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else", "except", "False", "finally",
                "for", "from", "if", "import", "in", "is", "lambda", "None", "not", "or", "pass", "raise", "return", "True", "try", "while", "with",
                "yield",
            ],
            Lang::Go => &[
                "break", "case", "chan", "const", "continue", "default", "defer", "else", "false", "for", "func", "go", "if", "import", "interface",
                "map", "nil", "package", "range", "return", "select", "struct", "switch", "true", "type", "var",
            ],
            Lang::Shell => &["case", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if", "in", "local", "return", "then", "while"],
            Lang::Json | Lang::Yaml | Lang::Toml => &["true", "false", "null"],
            _ => &[],
        }
    }
}
