//! Production-construct scan: counts forbidden constructs in production-reachable code and measures
//! the size budgets.
//!
//! Production-reachable means: not after the `#[cfg(test)]` attribute that opens a test module, and
//! not inside a file whose name or directory marks it as test code (`tests.rs`, `*/tests/*`). A
//! `#[cfg(test)]` that only gates `use` re-exports does not end the production region (`xlsx.rs`);
//! see [`production_lines`]. This is the measurement methodology `docs/HARDENING-PROGRAM.md` records,
//! so the ratchet in `tools/gate.sh` compares like with like.
//!
//! Emits JSON on stdout:
//!
//! ```text
//! {
//!   "crates": {"<crate dir>": {"assert_family": .., "panic": .., "expect": ..,
//!                              "unwrap": .., "unsafe": .., "indexing": .., "as_cast": ..,
//!                              "todo": .., "production_lines": .., "files": ..}},
//!   "structure": {"files_over_300_lines": ["<crate>:<path> (<lines>)", ..],
//!                 "functions_over_60_lines": ..,
//!                 "functions_over_60_sites": ["<crate>:<path>:<start>-<end> <fn> (<lines>)", ..],
//!                 "functions_over_25_logical_lines": ..,
//!                 "unstable_feature_sites": ["<crate>:<path>:<line> <feature>", ..]}
//! }
//! ```
//!
//! Keys sort alphabetically on the way out, which is what `json.dump(..., sort_keys=True)` did.

use crate::json::count;
use crate::paths;
use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

/// Unstable features this workspace's own source may gate on (the Holzman pinned-nightly policy).
///
/// The list is closed and must stay equal to the names the gate's `FEATURE_ALLOWLIST` passes to
/// `-Zallow-features` for our crates; adding a name is a deliberate edit in both places. The
/// dependency graph may name more (`proc-macro2` probes `proc_macro_span`, `anyhow` probes
/// `error_generic_member_access`), which is why the compiler flag cannot be the only enforcement: it
/// is crate-graph-wide, so it either fails on a dependency's probe or permits our source too. This
/// scan is the per-crate half of the policy, and it runs on our crates only.
const ALLOWED_FEATURES: [&str; 2] = ["portable_simd", "try_blocks"];

/// A crate-root feature gate: `#![feature(a, b)]`.
const FEATURE_GATE: &str = r"#!\[feature\(([^)]*)\)\]";

/// Line budget for one production `.rs` file.
const FILE_LINE_BUDGET: usize = 300;
/// Physical-line budget for one function.
const FN_LINE_BUDGET: usize = 60;
/// Logical-line budget for one function: non-blank lines that are not `//` comments.
const FN_LOGICAL_BUDGET: usize = 25;

/// The crates the scan covers, keyed exactly as `tools/quality-baseline.json` records them.
///
/// The set is closed on purpose: the baseline's `scan` and `structure` blocks know only these keys,
/// and a crate that appears here without a baseline entry reads as new debt for every count it
/// contributes (`scan <crate>.files: 0 -> 12`). Widening the scan is a baseline change
/// (`tools/gate.sh --update-baseline`), not a scan change.
const CRATES: [(&str, &str); 2] = [
    ("athletic-rust-pipeline", "src"),
    ("midwest-census", "crates/midwest-census/src"),
];

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

/// Scan both crates and write the report to stdout.
pub fn run() -> Result<()> {
    let rules = Rules::compile()?;
    let root = paths::repo_root();
    let mut crates: Map<String, Value> = Map::new();
    let mut over_300: Vec<String> = Vec::new();
    let mut over_60: Vec<String> = Vec::new();
    let mut features: Vec<String> = Vec::new();
    let mut over_logical = 0usize;

    for (name, relative_root) in CRATES {
        let mut crate_scan = CrateScan::default();
        for path in paths::rust_files(&root.join(relative_root))? {
            if is_test_file(&path) {
                continue;
            }
            let budgets = crate_scan.measure(&path, name, &rules, &mut over_300)?;
            over_60.extend(budgets.functions_over_60);
            over_logical = over_logical.saturating_add(budgets.functions_over_logical);
        }
        features.extend(crate_scan.feature_sites.iter().cloned());
        crates.insert(name.to_string(), Value::Object(crate_scan.into_counts()));
    }

    over_300.sort();
    over_60.sort();
    features.sort();
    println!(
        "{}",
        serde_json::to_string_pretty(&report(crates, over_300, over_60, features, over_logical))?
    );
    Ok(())
}

/// The whole report, as the JSON object the gate reads with `jq`.
fn report(
    crates: Map<String, Value>,
    over_300: Vec<String>,
    over_60: Vec<String>,
    features: Vec<String>,
    over_logical: usize,
) -> Value {
    let mut structure: Map<String, Value> = Map::new();
    structure.insert(
        "files_over_300_lines".to_string(),
        Value::Array(over_300.into_iter().map(Value::String).collect()),
    );
    structure.insert(
        "functions_over_60_lines".to_string(),
        Value::from(count(over_60.len())),
    );
    // The sites ride along as evidence: a budget the gate fails on has to name the function that
    // broke it, or the fix starts with a search instead of a read. They are not ratcheted — the count
    // above is the metric this scan certifies.
    structure.insert(
        "functions_over_60_sites".to_string(),
        Value::Array(over_60.into_iter().map(Value::String).collect()),
    );
    structure.insert(
        "unstable_feature_sites".to_string(),
        Value::Array(
            features
                .into_iter()
                .map(Value::String)
                .collect::<Vec<Value>>(),
        ),
    );
    structure.insert(
        "functions_over_25_logical_lines".to_string(),
        Value::from(count(over_logical)),
    );
    let mut report: Map<String, Value> = Map::new();
    report.insert("crates".to_string(), Value::Object(crates));
    report.insert("structure".to_string(), Value::Object(structure));
    Value::Object(report)
}

/// The function budgets one production file contributed.
struct Budgets {
    /// One entry per function over the physical budget: `<crate>:<path>:<start>-<end> <fn> (<lines>)`.
    functions_over_60: Vec<String>,
    functions_over_logical: usize,
}

/// One crate's accumulating counts.
struct CrateScan {
    counts: BTreeMap<String, u64>,
    production_lines: usize,
    files: u64,
    /// Feature gates outside [`ALLOWED_FEATURES`], as `<crate>:<path>:<line> <name>`.
    feature_sites: Vec<String>,
}

impl Default for CrateScan {
    /// A crate with every key at zero: the counts exist before any file is measured, so a crate whose
    /// source directory holds no production file still reports the full key set.
    fn default() -> Self {
        let mut counts: BTreeMap<String, u64> = FORBIDDEN
            .iter()
            .map(|(name, _)| ((*name).to_string(), 0))
            .collect();
        counts.insert("indexing".to_string(), 0);
        counts.insert("unstable_features".to_string(), 0);
        Self {
            counts,
            production_lines: 0,
            files: 0,
            feature_sites: Vec::new(),
        }
    }
}

impl CrateScan {
    /// Measure one production file, folding its counts in and recording its size-budget overruns.
    fn measure(
        &mut self,
        path: &Path,
        crate_name: &str,
        rules: &Rules,
        over_300: &mut Vec<String>,
    ) -> Result<Budgets> {
        self.files = self.files.saturating_add(1);
        let text = fs::read_to_string(path)
            .with_context(|| format!("reading {}", paths::relative(path)))?;
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        if lines.len() > FILE_LINE_BUDGET {
            over_300.push(format!(
                "{crate_name}:{} ({})",
                paths::relative(path),
                lines.len()
            ));
        }
        let production = production_lines(&lines, rules);
        self.production_lines = self.production_lines.saturating_add(production.len());
        for (name, pattern) in &rules.forbidden {
            let hits = production
                .iter()
                .filter(|line| pattern.is_match(line))
                .count();
            self.add(name, count(hits));
        }
        self.add("indexing", count_indexing(rules, &production));
        let label = format!("{crate_name}:{}", paths::relative(path));
        let features = unallowed_features(rules, &production, &label);
        self.add("unstable_features", count(features.len()));
        self.feature_sites.extend(features);
        let (functions_over_60, functions_over_logical) =
            scan_functions(rules, &production, &label);
        Ok(Budgets {
            functions_over_60,
            functions_over_logical,
        })
    }

    fn add(&mut self, name: &str, value: u64) {
        let slot = self.counts.entry(name.to_string()).or_insert(0);
        *slot = slot.saturating_add(value);
    }

    /// Finish the crate: totals in, as the JSON object the report carries.
    fn into_counts(self) -> Map<String, Value> {
        let mut counts = self.counts;
        counts.insert("production_lines".to_string(), count(self.production_lines));
        counts.insert("files".to_string(), self.files);
        counts
            .into_iter()
            .map(|(name, value)| (name, Value::from(value)))
            .collect()
    }
}

/// Files whose name or directory marks them as test code: never production-reachable.
fn is_test_file(path: &Path) -> bool {
    let named = path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.contains("tests"));
    let under_tests = path
        .components()
        .any(|part| part.as_os_str() == OsStr::new("tests"));
    named || under_tests
}

/// Lines before the `#[cfg(test)]` attribute that opens the file's test module.
///
/// A bare `#[cfg(test)]` ends the production region only when it gates a module. Files that
/// cfg(test)-gate `use` re-exports (`src/xlsx.rs`: `pub(crate) use cells::{..}`) keep every line in
/// scope, otherwise the real production code below them is invisible to every count and to the size
/// budgets.
fn production_lines(lines: &[String], rules: &Rules) -> Vec<String> {
    for (index, line) in lines.iter().enumerate() {
        if line.trim() != "#[cfg(test)]" {
            continue;
        }
        let mut lookahead = index.saturating_add(1);
        while let Some(candidate) = lines.get(lookahead) {
            if candidate.trim().is_empty() || rules.attribute.is_match(candidate) {
                lookahead = lookahead.saturating_add(1);
            } else {
                break;
            }
        }
        if lines
            .get(lookahead)
            .is_some_and(|candidate| rules.test_item.is_match(candidate))
        {
            return lines.get(..index).map_or_else(Vec::new, <[String]>::to_vec);
        }
    }
    lines.to_vec()
}

/// Count lines carrying real indexing, ignoring matches inside literals and comments.
fn count_indexing(rules: &Rules, lines: &[String]) -> u64 {
    let mut mask = CodeMask::default();
    let masked = mask.apply_all(lines, &rules.char_literal);
    count(
        masked
            .iter()
            .filter(|line| rules.indexing.is_match(line))
            .count(),
    )
}

/// Crate-root feature gates that name a feature outside [`ALLOWED_FEATURES`].
///
/// The names on one gate line are split on commas, so `#![feature(a, b)]` reports one entry per
/// disallowed name. A gate that spans lines cannot be read whole by a per-line scan; the names its
/// first line carries are still checked, and the metric is a violation count rather than a census.
fn unallowed_features(rules: &Rules, lines: &[String], label: &str) -> Vec<String> {
    let mut sites: Vec<String> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(captures) = rules.feature_gate.captures(line) else {
            continue;
        };
        let Some(names) = captures.get(1) else {
            continue;
        };
        for name in names.as_str().split(',') {
            let name = name.trim();
            if name.is_empty() || ALLOWED_FEATURES.contains(&name) {
                continue;
            }
            let at = index.saturating_add(1);
            sites.push(format!("{label}:{at} {name}"));
        }
    }
    sites
}

/// Functions over the physical and logical line budgets.
///
/// Braces are counted on masked lines. A `{` that is character-literal, string-literal or comment
/// text opens no block, and counting it left the walk's depth above zero for the rest of the file:
/// `profile/html/state.rs`'s `decode` measured an 85-line span because of `rest.find('{')`, and every
/// function after it was swallowed into that one span.
fn scan_functions(rules: &Rules, production: &[String], label: &str) -> (Vec<String>, usize) {
    let mut mask = CodeMask::default();
    let masked = mask.apply_all(production, &rules.char_literal);
    let mut over_60: Vec<String> = Vec::new();
    let mut over_logical = 0usize;
    let mut index = 0usize;
    while let Some(line) = production.get(index) {
        if !rules.function.is_match(line) {
            index = index.saturating_add(1);
            continue;
        }
        let span_end = body_end(&masked, index).saturating_add(1);
        let body = production
            .get(index..span_end.min(production.len()))
            .unwrap_or_default();
        let span = span_end.saturating_sub(index);
        if span > FN_LINE_BUDGET {
            let name = rules
                .function
                .captures(line)
                .and_then(|captures| captures.get(1))
                .map_or("<unnamed>", |name| name.as_str());
            let start = index.saturating_add(1);
            over_60.push(format!("{label}:{start}-{span_end} {name} ({span})"));
        }
        let logical = body
            .iter()
            .filter(|line| {
                let text = line.trim();
                !text.is_empty() && !text.starts_with("//")
            })
            .count();
        if logical > FN_LOGICAL_BUDGET {
            over_logical = over_logical.saturating_add(1);
        }
        index = span_end;
    }
    (over_60, over_logical)
}

/// The last line of the block that opens at `start`: the first line at or after it whose brace depth
/// returns to zero, having opened at least one brace.
fn body_end(masked: &[String], start: usize) -> usize {
    let mut depth = 0i64;
    let mut end = start;
    while let Some(line) = masked.get(end) {
        let opens = i64::try_from(line.matches('{').count()).unwrap_or(i64::MAX);
        let closes = i64::try_from(line.matches('}').count()).unwrap_or(i64::MAX);
        depth = depth.saturating_add(opens).saturating_sub(closes);
        let opened = masked
            .get(start..=end)
            .is_some_and(|span| span.iter().any(|line| line.contains('{')));
        if depth <= 0 && opened {
            break;
        }
        end = end.saturating_add(1);
    }
    end
}

/// Blanks Rust string literals and comments so the indexing count only sees code.
///
/// HTML, CSS and JavaScript payloads live in string literals here and they read like indexing to a
/// regex: `selector("a[href]")`, `headers[name] = value;` inside an injected script, and
/// `r"(?is)<t[hd]..` capture patterns. None of those are Rust indexing. Only the indexing metric and
/// the brace walk use the mask; every other pattern keeps the historical per-line semantics so their
/// recorded baselines do not move. Conservative by construction: anything not recognised as a
/// string, character literal or comment stays visible, so an unparsed form can only keep a hit, never
/// hide one. `/* */` depth and raw-string hashes carry across the lines of one file.
#[derive(Default)]
struct CodeMask {
    block_depth: u32,
    raw_hashes: Option<usize>,
}

impl CodeMask {
    /// Mask one line, carrying raw-string and block-comment state into the next line.
    fn apply(&mut self, line: &str, char_literal: &Regex) -> String {
        let chars: Vec<char> = line.chars().collect();
        let mut masked = chars.clone();
        let mut index = 0usize;
        while index < chars.len() {
            index = if self.raw_hashes.is_some() {
                self.end_raw_string(&chars, &mut masked, index)
            } else if self.block_depth > 0 {
                self.end_block_comment(&chars, &mut masked, index)
            } else {
                self.advance(&chars, &mut masked, index, char_literal)
            };
        }
        masked.into_iter().collect()
    }

    fn apply_all(&mut self, lines: &[String], char_literal: &Regex) -> Vec<String> {
        lines
            .iter()
            .map(|line| self.apply(line, char_literal))
            .collect()
    }

    /// Mask whatever construct starts at `index`, or step one character past it.
    fn advance(
        &mut self,
        line: &[char],
        masked: &mut [char],
        index: usize,
        char_literal: &Regex,
    ) -> usize {
        match line.get(index).copied() {
            Some('/') if starts_with(line, index, &['/', '/']) => {
                blank(masked, index, line.len());
                line.len()
            }
            Some('/') if starts_with(line, index, &['/', '*']) => {
                self.block_depth = 1;
                blank(masked, index, index.saturating_add(2));
                self.end_block_comment(line, masked, index.saturating_add(2))
            }
            Some('r') if !continues_identifier(line, index) => {
                self.start_raw_string(line, masked, index)
            }
            Some('"') => self.end_string(line, masked, index),
            Some('\'') => self.end_char_literal(line, masked, index, char_literal),
            _ => index.saturating_add(1),
        }
    }

    fn start_raw_string(&mut self, line: &[char], masked: &mut [char], index: usize) -> usize {
        let mut cursor = index.saturating_add(1);
        while line.get(cursor) == Some(&'#') {
            cursor = cursor.saturating_add(1);
        }
        if line.get(cursor) != Some(&'"') {
            return index.saturating_add(1);
        }
        self.raw_hashes = Some(cursor.saturating_sub(index).saturating_sub(1));
        blank(masked, index, cursor.saturating_add(1));
        self.end_raw_string(line, masked, cursor.saturating_add(1))
    }

    fn end_raw_string(&mut self, line: &[char], masked: &mut [char], index: usize) -> usize {
        let terminator = terminator(self.raw_hashes.unwrap_or(0));
        match find_from(line, index, &terminator) {
            Some(end) => {
                self.raw_hashes = None;
                let after = end.saturating_add(terminator.len());
                blank(masked, index, after);
                after
            }
            None => {
                blank(masked, index, line.len());
                line.len()
            }
        }
    }

    fn end_block_comment(&mut self, line: &[char], masked: &mut [char], index: usize) -> usize {
        let start = index;
        let mut cursor = index;
        while cursor < line.len() {
            let nested = find_from(line, cursor, &['/', '*']);
            let close = find_from(line, cursor, &['*', '/']);
            if opens_first(nested, close) {
                let Some(next) = nested else { break };
                self.block_depth = self.block_depth.saturating_add(1);
                cursor = next.saturating_add(2);
                continue;
            }
            let Some(end) = close else { break };
            self.block_depth = self.block_depth.saturating_sub(1);
            if self.block_depth == 0 {
                let after = end.saturating_add(2);
                blank(masked, start, after);
                return after;
            }
            cursor = end.saturating_add(2);
        }
        blank(masked, start, line.len());
        line.len()
    }

    fn end_string(&mut self, line: &[char], masked: &mut [char], index: usize) -> usize {
        let mut cursor = index.saturating_add(1);
        while let Some(char) = line.get(cursor).copied() {
            if char == '\\' {
                cursor = cursor.saturating_add(2);
            } else if char == '"' {
                let after = cursor.saturating_add(1);
                blank(masked, index, after);
                return after;
            } else {
                cursor = cursor.saturating_add(1);
            }
        }
        blank(masked, index, line.len());
        line.len()
    }

    fn end_char_literal(
        &mut self,
        line: &[char],
        masked: &mut [char],
        index: usize,
        char_literal: &Regex,
    ) -> usize {
        let tail: String = line.get(index..).unwrap_or_default().iter().collect();
        let Some(literal) = char_literal.find(&tail) else {
            return index.saturating_add(1);
        };
        let consumed = tail
            .get(..literal.end())
            .map_or(0, |matched| matched.chars().count());
        let after = index.saturating_add(consumed);
        blank(masked, index, after);
        after
    }
}

/// Whether the next nested comment opener wins over the next comment closer.
///
/// This is `production_scan.py`'s `0 <= nested < close or close < 0` spelled out: a nested `/*` wins
/// unless a `*/` comes first, and a `*/` with no `/*` left on the line always closes. Neither present
/// ends the walk.
fn opens_first(nested: Option<usize>, close: Option<usize>) -> bool {
    match (nested, close) {
        (Some(next), Some(end)) => next < end,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

/// The terminator that closes a raw string with `hashes` hashes: `"` and then that many `#`.
fn terminator(hashes: usize) -> Vec<char> {
    let mut terminator: Vec<char> = Vec::with_capacity(hashes.saturating_add(1));
    terminator.push('"');
    terminator.resize(hashes.saturating_add(1), '#');
    terminator
}

/// Whether `needle` occurs at `index`.
fn starts_with(line: &[char], index: usize, needle: &[char]) -> bool {
    needle
        .iter()
        .enumerate()
        .all(|(offset, expected)| line.get(index.saturating_add(offset)) == Some(expected))
}

/// The first position at or after `index` where `needle` occurs.
fn find_from(line: &[char], index: usize, needle: &[char]) -> Option<usize> {
    let first = *needle.first()?;
    let mut position = index;
    while let Some(char) = line.get(position) {
        if *char == first && starts_with(line, position, needle) {
            return Some(position);
        }
        position = position.saturating_add(1);
    }
    None
}

/// Whether the character before `index` continues an identifier, which makes an `r` an identifier's
/// last letter rather than the start of a raw string.
fn continues_identifier(line: &[char], index: usize) -> bool {
    index > 0
        && line
            .get(index.saturating_sub(1))
            .is_some_and(|char| char.is_alphanumeric() || *char == '_')
}

/// Blank `masked[start..end]`, clamped to the line's length.
fn blank(masked: &mut [char], start: usize, end: usize) {
    let limit = end.min(masked.len());
    let mut position = start;
    while position < limit {
        if let Some(slot) = masked.get_mut(position) {
            *slot = ' ';
        }
        position = position.saturating_add(1);
    }
}

/// Every pattern the scan matches, compiled once.
struct Rules {
    forbidden: Vec<(&'static str, Regex)>,
    indexing: Regex,
    char_literal: Regex,
    attribute: Regex,
    test_item: Regex,
    function: Regex,
    feature_gate: Regex,
}

impl Rules {
    fn compile() -> Result<Self> {
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
}

/// Compile one pattern, naming it when it does not.
pub fn compile(pattern: &str) -> Result<Regex> {
    Regex::new(pattern).with_context(|| format!("compiling pattern {pattern:?}"))
}
