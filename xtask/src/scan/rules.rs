use anyhow::{Context, Result};
use regex::Regex;

const ALLOWED_FEATURES: [&str; 2] = ["portable_simd", "try_blocks"];

const FEATURE_GATE: &str = r"^\s*#!\[feature\(([^)]*)\)\]";

const FORBIDDEN: [(&str, &str); 9] = [
    ("unsafe", r"\bunsafe\s*(\{|fn|impl|trait|extern)"),
    ("unwrap", r"\.unwrap\(\)"),
    ("expect", r"\.expect\("),
    ("panic", r"\bpanic!\("),
    ("unreachable", r"\bunreachable!\("),
    ("todo", r"\b(todo!|unimplemented!)\("),
    ("assert_family", r"\b(assert!|assert_eq!|assert_ne!)\("),
    ("dbg", r"\bdbg!\("),
    (
        "as_cast",
        r"\bas\s+(u8|u16|u32|u64|usize|i8|i16|i32|i64|isize|f32|f64)\b",
    ),
];

const INDEXING: &str = r"[A-Za-z0-9_)\]]\[\s*[a-zA-Z0-9_]+\s*\]";

const CHAR_LITERAL: &str = r"^(?:'\\(?:.|x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f]{1,6}\})'|'.')";

const ATTRIBUTE_LINE: &str = r"^\s*#\[";

const TEST_ITEM_LINE: &str = r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s";

pub(crate) struct Rules {
    pub(crate) forbidden: Vec<(&'static str, Regex)>,
    pub(crate) indexing: Regex,
    pub(crate) char_literal: Regex,
    pub(crate) attribute: Regex,
    pub(crate) test_item: Regex,
    feature_gate: Regex,
}

impl Rules {
    pub(crate) fn compile() -> Result<Self> {
        let mut forbidden = Vec::with_capacity(FORBIDDEN.len());
        for (name, pattern) in FORBIDDEN {
            forbidden.push((name, compile(pattern)?));
        }
        Ok(Self {
            forbidden,
            indexing: compile(INDEXING)?,
            char_literal: compile(CHAR_LITERAL)?,
            attribute: compile(ATTRIBUTE_LINE)?,
            test_item: compile(TEST_ITEM_LINE)?,
            feature_gate: compile(FEATURE_GATE)?,
        })
    }

    pub(crate) fn unallowed_features(&self, lines: &[String]) -> Vec<(usize, String)> {
        let mut sites: Vec<(usize, String)> = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            let names = self
                .feature_gate
                .captures(line)
                .and_then(|captures| captures.get(1));
            let Some(names) = names else {
                continue;
            };
            for name in names.as_str().split(',') {
                let name = name.trim();
                if name.is_empty() || ALLOWED_FEATURES.contains(&name) {
                    continue;
                }
                sites.push((index.saturating_add(1), name.to_string()));
            }
        }
        sites
    }
}

pub(crate) fn compile(pattern: &str) -> Result<Regex> {
    Regex::new(pattern).with_context(|| format!("compiling pattern {pattern:?}"))
}
