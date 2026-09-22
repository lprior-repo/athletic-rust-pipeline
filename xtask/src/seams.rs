//! Module-seam enforcement for the census crate.
//!
//! `docs/HARDENING-PROGRAM.md` §9 keeps the census a single crate with module seams, so the
//! compiler seals items (private submodules, visibility) but cannot forbid an edge between
//! top-level modules. This measurer reads every production `.rs` file under
//! `crates/midwest-census/src`, resolves each `crate::…` reference to its top-level module, and
//! compares the `(from, to)` pair against [`ALLOWED`]. A pair outside the table is a violation:
//! the walker names the file and line, prints the full report as JSON on stdout, and fails.
//!
//! The table is the ratchet. Adding an edge is a deliberate edit here; deleting a row makes that
//! edge a violation again, because the check fails closed. `(sources, store)` is listed although
//! `ARCHITECTURE.md` calls it a direction violation — `AdapterContext` carries `&Store` today.
//! When the adapters return entity batches instead, delete the row and the walker starts
//! enforcing the narrower graph. `spawn` is the region's task spawner: it reads the clock for its
//! drain deadline and classifies completion through the outcome lattice, and the two modules that
//! own a region (`bootstrap`, `restate_services`) start their tasks through it.
//!
//! Comment lines and test code are out of scope: files named `tests.rs`, files under a `tests/`
//! directory, and the region after the `#[cfg(test)]` that opens a module, because none of them
//! can reach production callers.
//!
//! Emits JSON on stdout:
//!
//! ```text
//! {
//!   "modules": ["bests", ...],
//!   "edges": [{"from": "sources", "to": "store", "refs": 24}, ...],
//!   "violations": [{"from": "sources", "to": "report", "file": "...", "line": 283}, ...]
//! }
//! ```

use crate::json::count;
use crate::paths;
use anyhow::{bail, Context, Result};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Allowed edges between top-level modules of `crates/midwest-census/src`, as `(from, to)`.
///
/// The direction rule is `ARCHITECTURE.md`'s: adapters and workflows depend on domain types and
/// on the store, never the reverse; `net` and `school_index` are leaves; `store` and `report` may
/// not reach into `net` (the clock lives in `clock`).
const ALLOWED: &[(&str, &str)] = &[
    ("bests", "report"),
    ("bests", "store"),
    ("bootstrap", "clock"),
    ("bootstrap", "outcome"),
    ("bootstrap", "restate_services"),
    ("bootstrap", "spawn"),
    ("bootstrap", "store"),
    ("census", "clock"),
    ("census", "net"),
    ("census", "report"),
    ("census", "school_index"),
    ("census", "sources"),
    ("census", "store"),
    ("net", "clock"),
    ("outcome", "restate_services"),
    ("report", "clock"),
    ("report", "store"),
    ("restate_services", "bests"),
    ("restate_services", "census"),
    ("restate_services", "clock"),
    // ARCHITECTURE.md §1: the batch path and the durable path share the adapters, the store and the
    // reports, and differ only in who owns the journal. The workflow layer therefore names the
    // adapter error type it classifies into the durable retry policy (`jobs::collect_error`) and the
    // polite fetcher the jurisdiction object holds for the whole process, which is also what keeps
    // one remote origin drawing from one admission budget (§3).
    ("restate_services", "net"),
    ("restate_services", "sources"),
    ("restate_services", "outcome"),
    ("restate_services", "report"),
    ("restate_services", "spawn"),
    ("restate_services", "store"),
    ("restate_services", "workbook"),
    ("sources", "net"),
    ("sources", "school_index"),
    ("sources", "store"),
    ("spawn", "clock"),
    ("spawn", "outcome"),
    ("store", "clock"),
    ("workbook", "bests"),
    ("workbook", "report"),
    // The meta sheets render the adapter surface itself — slug, transport, declared capabilities
    // and the per-origin request cost — so the workbook reads the registry table as data. It never
    // calls an adapter and never fetches.
    ("workbook", "sources"),
    ("workbook", "store"),
];

/// `crate::` — the only prefix that points into this crate's own module tree.
const PREFIX: [char; 7] = ['c', 'r', 'a', 't', 'e', ':', ':'];

/// One resolved reference: source module, target top module, and where it sits.
#[derive(Debug)]
struct Reference {
    from: String,
    to: String,
    file: String,
    line: usize,
}

/// Scan the census source tree, print the report, and fail on any disallowed edge.
pub fn run() -> Result<()> {
    let src = paths::census_crate().join("src");
    let mut modules = BTreeSet::new();
    let mut references = Vec::new();
    for file in paths::rust_files(&src)? {
        if is_test_file(&file) {
            continue;
        }
        let Some(from) = top_module(&src, &file) else {
            continue;
        };
        modules.insert(from.clone());
        let text = fs::read_to_string(&file)
            .with_context(|| format!("reading {}", paths::relative(&file)))?;
        let lines: Vec<&str> = text.lines().collect();
        let production_end = test_module_start(&lines);
        for (index, line) in lines.iter().enumerate() {
            if index >= production_end {
                break;
            }
            for to in refs_in_line(line) {
                if to == from {
                    continue;
                }
                references.push(Reference {
                    from: from.clone(),
                    to,
                    file: paths::relative(&file),
                    line: index.saturating_add(1),
                });
            }
        }
    }

    let mut grouped: BTreeMap<(String, String), Vec<&Reference>> = BTreeMap::new();
    for reference in &references {
        grouped
            .entry((reference.from.clone(), reference.to.clone()))
            .or_default()
            .push(reference);
    }

    let mut edges = Vec::new();
    let mut violations = Vec::new();
    for ((from, to), group) in &grouped {
        edges.push(json!({"from": from, "to": to, "refs": count(group.len())}));
        if !is_allowed(from, to) {
            for reference in group {
                violations.push(json!({
                    "from": from,
                    "to": to,
                    "file": reference.file,
                    "line": count(reference.line),
                }));
            }
        }
    }
    let violation_count = violations.len();
    let report = json!({
        "modules": modules,
        "edges": edges,
        "violations": violations,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    if violation_count > 0 {
        bail!("{violation_count} module-seam violation(s): an edge outside the allowed table");
    }
    Ok(())
}

/// Whether `(from, to)` is a declared edge of the census module graph.
fn is_allowed(from: &str, to: &str) -> bool {
    ALLOWED
        .iter()
        .any(|(allowed_from, allowed_to)| *allowed_from == from && *allowed_to == to)
}

/// The top-level module a source file belongs to, `None` for bin-tree files.
///
/// `src/store/read.rs` and `src/store/mod.rs` are both `store`; a file directly under `src/`
/// (`lib.rs`, `bootstrap.rs`) is its own stem. `main.rs`, `cli/**` and `bin/**` are excluded:
/// every binary is its own crate root, so those files consume the library through its public
/// API (`midwest_census::…`) and cannot form a lib-internal seam.
fn top_module(src: &Path, file: &Path) -> Option<String> {
    let relative = file.strip_prefix(src).ok()?;
    let mut components = relative.components();
    let first = components.next()?.as_os_str().to_str()?;
    if first == "main.rs" || first == "cli" || first == "bin" {
        return None;
    }
    if components.next().is_none() {
        return relative
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_string);
    }
    Some(first.to_string())
}

/// Files whose name or directory marks them as test code: never production-reachable.
fn is_test_file(path: &Path) -> bool {
    path.file_stem().is_some_and(|stem| stem == "tests")
        || path
            .components()
            .any(|component| component.as_os_str() == "tests")
}

/// The line index where a file's production region ends: the `mod` item gated by `#[cfg(test)]`.
///
/// A bare `#[cfg(test)]` is only a cut when it opens a module; one that gates `use` re-exports
/// (the `xlsx.rs` shape the production scan documents) keeps the file in scope, so the walk
/// continues past it. The search is bounded to the few attribute lines a `mod` item can carry.
fn test_module_start(lines: &[&str]) -> usize {
    for (index, line) in lines.iter().enumerate() {
        if !line.trim_start().starts_with("#[cfg(test)]") {
            continue;
        }
        for (probe, next) in lines.iter().enumerate().skip(index.saturating_add(1)) {
            let trimmed = next.trim_start();
            if trimmed.is_empty() || trimmed.starts_with("#[") {
                if probe.saturating_sub(index) > 4 {
                    break;
                }
                continue;
            }
            if mod_item(trimmed) {
                return probe;
            }
            break;
        }
    }
    lines.len()
}

/// Whether a line declares a module item, after any `pub`/`pub(...)` prefix.
fn mod_item(line: &str) -> bool {
    let mut rest = line;
    while let Some(after) = rest.strip_prefix("pub") {
        rest = after.trim_start();
        if let Some(inner) = rest.strip_prefix('(') {
            let Some(close) = inner.find(')') else {
                return false;
            };
            rest = inner
                .get(close.saturating_add(1)..)
                .unwrap_or("")
                .trim_start();
        }
    }
    rest.strip_prefix("mod")
        .is_some_and(|after| after.is_empty() || after.starts_with(char::is_whitespace))
}

/// Every top-level module a line references through `crate::…`.
///
/// A `crate::{a::A, b::B}` group contributes each item's first segment; a plain path contributes
/// its first segment. References that sit inside a string literal are skipped with a quote-parity
/// approximation — good enough for a ratchet, and it can only over-report, never hide.
fn refs_in_line(line: &str) -> Vec<String> {
    if line.trim_start().starts_with("//") {
        return Vec::new();
    }
    let chars: Vec<char> = line.chars().collect();
    let mut targets = Vec::new();
    let mut cursor = 0usize;
    while let Some(position) = find_from(&chars, &PREFIX, cursor) {
        cursor = position.saturating_add(PREFIX.len());
        if inside_string(&chars, position) {
            continue;
        }
        let rest = chars.get(cursor..).unwrap_or_default();
        if rest.first() == Some(&'{') {
            group_targets(rest, &mut targets);
        } else if let Some(name) = leading_ident(rest) {
            push_target(name, &mut targets);
        }
    }
    targets
}

/// The module names a brace group contributes: each item's first segment.
///
/// `{store::Store, report::Error}` contributes `store` and `report`; nesting contributes the outer
/// first segment (`{store::{Store, Table}}` contributes `store`), which is what a top-level table
/// needs.
fn group_targets(chars: &[char], targets: &mut Vec<String>) {
    let mut depth = 0usize;
    let mut item: Vec<char> = Vec::new();
    for ch in chars {
        match ch {
            '{' => {
                depth = depth.saturating_add(1);
                if depth > 1 {
                    item.push(*ch);
                }
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    push_item(&item, targets);
                    return;
                }
                item.push(*ch);
            }
            ',' if depth == 1 => {
                push_item(&item, targets);
                item.clear();
            }
            _ => item.push(*ch),
        }
    }
}

/// One group item's contribution, dropped when it names the group rather than a module.
fn push_item(item: &[char], targets: &mut Vec<String>) {
    if let Some(name) = leading_ident(item) {
        push_target(name, targets);
    }
}

/// Record a target unless it is a path keyword rather than a module.
fn push_target(name: String, targets: &mut Vec<String>) {
    if name != "self" && name != "super" && name != "crate" {
        targets.push(name);
    }
}

/// The leading identifier of a path fragment, raw-string prefix stripped, or `None`.
fn leading_ident(chars: &[char]) -> Option<String> {
    let mut rest = chars;
    while rest.first().is_some_and(|ch| ch.is_whitespace()) {
        rest = rest.get(1..)?;
    }
    if rest.first() == Some(&'r') && rest.get(1) == Some(&'#') {
        rest = rest.get(2..)?;
    }
    let mut name = String::new();
    for ch in rest {
        if ch.is_ascii_alphanumeric() || *ch == '_' {
            name.push(*ch);
        } else {
            break;
        }
    }
    match name.chars().next() {
        Some(first) if !first.is_ascii_digit() => Some(name),
        _ => None,
    }
}

/// Whether the character at `at` sits inside a simple string literal.
fn inside_string(chars: &[char], at: usize) -> bool {
    let mut open = false;
    let mut previous = ' ';
    for ch in chars.iter().take(at) {
        if *ch == '"' && previous != '\\' {
            open = !open;
        }
        previous = *ch;
    }
    open
}

/// The first position at or after `start` where `needle` occurs.
fn find_from(hay: &[char], needle: &[char], start: usize) -> Option<usize> {
    let width = needle.len();
    let last = hay.len().checked_sub(width)?;
    (start..=last).find(|index| hay.get(*index..index.saturating_add(width)) == Some(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_modules_read_the_path_shape() {
        let src = Path::new("/crates/midwest-census/src");
        assert_eq!(top_module(src, &src.join("lib.rs")).as_deref(), Some("lib"));
        assert_eq!(
            top_module(src, &src.join("bootstrap/mod.rs")).as_deref(),
            Some("bootstrap")
        );
        assert_eq!(
            top_module(src, &src.join("sources/wiaa/collect.rs")).as_deref(),
            Some("sources")
        );
        assert_eq!(top_module(src, &src.join("main.rs")), None);
        assert_eq!(top_module(src, &src.join("cli/mod.rs")), None);
        assert_eq!(top_module(src, &src.join("bin/midwest-serve.rs")), None);
    }

    #[test]
    fn references_resolve_to_top_level_modules() {
        assert_eq!(
            refs_in_line("use crate::store::{Store, Table};"),
            vec!["store".to_string()]
        );
        assert_eq!(
            refs_in_line("let now = crate::net::now_iso8601();"),
            vec!["net".to_string()]
        );
        assert_eq!(
            refs_in_line("use crate::{self as census_crate, store::Store};"),
            vec!["store".to_string()]
        );
        assert!(refs_in_line("// crate::store::Store").is_empty());
        assert!(refs_in_line("let s = \"crate::report::read_rows\";").is_empty());
        assert_eq!(
            refs_in_line("use crate::{report::Error, sources::AdapterReport};"),
            vec!["report".to_string(), "sources".to_string()]
        );
    }

    #[test]
    fn test_modules_end_the_production_region() {
        let lines = [
            "fn production() {}",
            "#[cfg(test)]",
            "mod tests {",
            "    use crate::report::read_rows;",
            "}",
        ];
        assert_eq!(test_module_start(&lines), 2);
        let export_gate = [
            "pub(crate) use cells::{A, B};",
            "#[cfg(test)]",
            "pub(crate) use test_support::X;",
            "fn production() {}",
        ];
        assert_eq!(test_module_start(&export_gate), export_gate.len());
    }

    #[test]
    fn files_under_tests_are_test_code() {
        assert!(is_test_file(Path::new("src/store/tests.rs")));
        assert!(is_test_file(Path::new("src/sources/tests/fixtures.rs")));
        assert!(!is_test_file(Path::new("src/store/read.rs")));
    }
}
