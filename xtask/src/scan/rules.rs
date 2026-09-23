//! The scan's patterns, compiled once.
//!
//! Each pattern carries the reason it is spelled the way it is, because every one of them has been
//! loosened or tightened in response to a false positive that cost someone a debugging session. Two
//! rules hold throughout: a pattern may only ever match text a Rust compiler would accept as the
//! construct it names, and prose that merely *describes* the construct (a doc comment quoting a
//! gate, a test asserting on a sentence) must not be scored as one.

use anyhow::{Context, Result};
use regex::Regex;

/// Unstable features this workspace's own source may gate on (the Holzman pinned-nightly policy).
///
/// The list is closed and must stay equal to the names the gate's `FEATURE_ALLOWLIST` passes to
/// `-Zallow-features` for our crates; adding a name is a deliberate edit in both places. The
/// dependency graph may name more (`proc-macro2` probes `proc_macro_span`, `anyhow` probes
/// `error_generic_member_access`), which is why the compiler flag cannot be the only enforcement: it
/// is crate-graph-wide, so it either fails on a dependency's probe or permits our source too. This
/// scan is the per-crate half of the policy, and it runs on our crates only.
const ALLOWED_FEATURES: [&str; 2] = ["portable_simd", "try_blocks"];

/// A crate-root feature gate: `#![feature(a, b)]` written as the inner attribute it is.
///
/// Anchored at the start of a line, because that is the only place the construct can legally appear:
/// `#![feature(..)]` is an inner attribute, the compiler accepts it at the crate root alone, and
/// rustfmt writes it at column 0. Without the anchor the pattern also matches *prose* that describes
/// a gate — a doc comment reading ``/// A crate-root feature gate: `#![feature(a, b)]`.`` scored
/// `a` and `b` as two disallowed features, which is how this crate reported four `unstable_features`
/// it never had. Nothing is hidden by the anchor: an indented gate still matches (`\s*`), and a gate
/// cannot appear mid-line at all.
const FEATURE_GATE: &str = r"^\s*#!\[feature\(([^)]*)\)\]";

/// Forbidden-construct patterns, exactly as the deleted `production_scan.py` declared them.
///
/// Every pattern is a per-line search: one line that carries the construct counts once, however
/// often it repeats it. The other metrics keep that per-line semantics too, so their recorded
/// baselines do not move for reasons unrelated to the code.
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

/// Real Rust indexing: `expr[i]`, `expr[0]`, `expr[identifier]`, where the `[` follows an expression.
///
/// The left context requires an expression-ending character (identifier char, `)`, or `]`), which
/// deliberately excludes slice and array types (`&[T]`, `&'a [T]`, `&mut [u8]`, `: [T]`, `-> [T]`,
/// `[u8; 4]`), attributes (`#[must_use]`, `#[test]`, `#[cfg(test)]`) and prose or JSON (`: [1, 2]`).
/// Whitespace before `[` is not allowed on purpose: the gate runs rustfmt, so `expr [i]` cannot occur
/// in this repository, while allowing the space re-admits every type false positive above by letting
/// the match start on the space itself (`&mut [u8]` matched once the engine began at the space after
/// `mut`). Do not "simplify" this back to `\[\s*[a-zA-Z0-9_]+\s*\]`: that pattern scored slice types
/// and `#[must_use]` as indexing (`src/domain/decision.rs`: 21 reported, 0 real index operators).
///
/// `production_scan.py` spelled the left context as a lookbehind, which the `regex` crate does not
/// support. Folding the same character class into the match is equivalent for this yes/no test: it can
/// only ever match the single character before `[`, so a match of this form implies a match of the
/// lookbehind form and the other way round.
const INDEXING: &str = r"[A-Za-z0-9_)\]]\[\s*[a-zA-Z0-9_]+\s*\]";

/// A Rust character literal: `'a'`, `'\n'`, `'\x41'`, `'\u{7f}'`.
///
/// Anchored with `^` because the deleted script asked for a match *at* the position it had reached
/// (`CHAR_LITERAL.match(line, index)`); an unanchored search over the rest of the line could jump
/// forward to a later literal and over-blank the line.
const CHAR_LITERAL: &str = r"^(?:'\\(?:.|x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f]{1,6}\})'|'.')";

/// `#[...]` on its own line, possibly indented: an attribute between `#[cfg(test)]` and its item.
const ATTRIBUTE_LINE: &str = r"^\s*#\[";

/// The `mod` item a `#[cfg(test)]` has to gate for the file's production region to end there.
const TEST_ITEM_LINE: &str = r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s";

/// A function definition, with the optional `pub`, `const`, `async` and `unsafe` prefixes a real
/// signature carries.
const FUNCTION: &str =
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|async\s+|unsafe\s+)*fn\s+([a-zA-Z0-9_]+)";

/// Every pattern the scan matches, compiled once.
pub(crate) struct Rules {
    pub(crate) forbidden: Vec<(&'static str, Regex)>,
    pub(crate) indexing: Regex,
    pub(crate) char_literal: Regex,
    pub(crate) attribute: Regex,
    pub(crate) test_item: Regex,
    pub(crate) function: Regex,
    feature_gate: Regex,
}

impl Rules {
    /// Compile every pattern in the table, naming the first one that does not.
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
            function: compile(FUNCTION)?,
            feature_gate: compile(FEATURE_GATE)?,
        })
    }

    /// The disallowed feature names one crate-root gate line carries, with their line numbers.
    ///
    /// The names on one gate are split on commas, so `#![feature(a, b)]` reports one entry per
    /// disallowed name. A gate that spans lines cannot be read whole by a per-line scan; the names
    /// its first line carries are still checked, and the metric is a violation count rather than a
    /// census.
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

/// Compile one pattern, naming it when it does not.
pub(crate) fn compile(pattern: &str) -> Result<Regex> {
    Regex::new(pattern).with_context(|| format!("compiling pattern {pattern:?}"))
}
