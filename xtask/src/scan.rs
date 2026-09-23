//! Production-construct scan: counts forbidden constructs in production-reachable code and measures
//! the size budgets, over every package the workspace declares.
//!
//! The package set comes from `cargo metadata --no-deps` ([`packages`]), so a crate that joins the
//! workspace joins the scan; a repository that keeps the list by hand scans what the hand remembers.
//! For each package the scan walks `<manifest>/src` plus, where they exist, `<manifest>/benches`,
//! `<manifest>/kani` and a nested cargo-fuzz `fuzz/fuzz_targets`.
//!
//! Production-reachable means: not after the `#[cfg(test)]` attribute that opens a test module, not
//! inside a file whose name or directory marks it as test code (`tests.rs`, `*/tests/*`), and not
//! inside a harness root (`examples/`, `benches/`, `kani/`, `fuzz/fuzz_targets/`) — a benchmark target has no
//! error channel and a Kani harness asserts by construction, exactly as the gate's own strict clippy
//! lane draws the line at `--lib --bins --examples`. Harness files are still read: they are measured
//! for size, so a 400-line benchmark or a 200-line proof harness is visible, but their constructs are
//! not counted as production debt. A `#[cfg(test)]` that only gates `use` re-exports does not end the
//! production region (`xlsx.rs`); see [`counts::production_lines`]. This is the measurement
//! methodology `docs/HARDENING-PROGRAM.md` records, so the ratchet in `tools/gate.sh` compares like
//! with like.
//!
//! Emits JSON on stdout, and the package list it covered on stderr:
//!
//! ```text
//! {
//!   "packages": ["<package>", ..],
//!   "crates": {"<package>": {"assert_family": .., "panic": .., "expect": ..,
//!                            "unwrap": .., "unsafe": .., "indexing": .., "as_cast": ..,
//!                            "todo": .., "production_lines": .., "files": ..}},
//!   "structure": {"files_over_300_lines": ["<package>:<path> (<lines>)", ..],
//!                 "functions_over_60_lines": ..,
//!                 "functions_over_60_sites": ["<package>:<path>:<start>-<end> <fn> (<lines>)", ..],
//!                 "functions_over_25_logical_lines": ..,
//!                 "unstable_feature_sites": ["<package>:<path>:<line> <feature>", ..]}
//! }
//! ```
//!
//! `crates` holds one entry per package, whether or not that package has a file to measure, so a
//! package that stops being scanned is visible as a missing key rather than as silence. `files` counts
//! every file read for the package, harness files included; `production_lines` counts the
//! production-reachable lines among them. Keys sort alphabetically on the way out, which is what
//! `json.dump(..., sort_keys=True)` did.

pub(crate) mod counts;
mod mask;
mod packages;
pub(crate) mod rules;

use crate::json::count;
use crate::paths;
use anyhow::{Context, Result};
use counts::CrateScan;
use packages::{Package, Root};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) use counts::is_test_file;
pub(crate) use packages::members;
pub(crate) use rules::{compile, Rules};

/// One file the scan reads: the package that owns it, its path, and whether it is harness code.
pub(crate) struct SourceFile {
    pub(crate) package: String,
    pub(crate) path: PathBuf,
    pub(crate) harness: bool,
}

impl SourceFile {
    /// `<package>:<path relative to the repository root>`: how every report entry names this file.
    pub(crate) fn label(&self) -> String {
        format!("{}:{}", self.package, paths::relative(&self.path))
    }
}

/// Scan every first-party package and write the report to stdout.
pub(crate) fn run() -> Result<()> {
    let (packages, files) = walk()?;
    let measured = measure(&packages, &files)?;
    announce(&measured.packages);
    println!("{}", serde_json::to_string_pretty(&report_value(measured))?);
    Ok(())
}

/// The same report as a value, for a caller that has to read it rather than print it.
pub(crate) fn report() -> Result<Value> {
    let (packages, files) = walk()?;
    Ok(report_value(measure(&packages, &files)?))
}

/// Every file the scan covers: the roots of every first-party package, minus test files.
pub(crate) fn source_files() -> Result<Vec<SourceFile>> {
    Ok(walk()?.1)
}

/// The production-reachable lines of one file, masked.
///
/// Masking is what lets a pattern match Rust syntax rather than prose about it: a doc comment that
/// quotes a construct, or a string literal holding a fragment of HTML, is blanked before the pattern
/// runs. A metric that reads syntax should use this; the per-line construct counts deliberately keep
/// their historical unmasked semantics, so their recorded baselines do not move.
pub(crate) fn masked_production(file: &SourceFile, rules: &Rules) -> Result<Vec<String>> {
    let lines = read_lines(&file.path)?;
    let production = counts::production_lines(&lines, rules);
    let mut mask = mask::CodeMask::default();
    Ok(mask.apply_all(&production, &rules.char_literal))
}

/// The lines of one scanned file, naming it when it cannot be read.
pub(crate) fn read_lines(path: &Path) -> Result<Vec<String>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading {}", paths::relative(path)))?;
    Ok(text.lines().map(str::to_string).collect())
}

/// The package set and every file of theirs the scan reads, from one metadata query.
fn walk() -> Result<(Vec<Package>, Vec<SourceFile>)> {
    let packages = packages::scanned()?;
    let mut files = Vec::new();
    for package in &packages {
        for root in &package.roots {
            files.extend(root_files(&package.name, root)?);
        }
    }
    Ok((packages, files))
}

/// The scanned files of one root, skipping the ones that name themselves test code.
fn root_files(package: &str, root: &Root) -> Result<Vec<SourceFile>> {
    // A root is a directory to walk, except for a package's build script, which is a single file.
    let paths = if root.path.is_file() {
        vec![root.path.clone()]
    } else {
        paths::rust_files(&root.path)?
    };
    let mut files = Vec::new();
    for path in paths {
        if counts::is_test_file(&path) {
            continue;
        }
        files.push(SourceFile {
            package: package.to_string(),
            path,
            harness: root.harness,
        });
    }
    Ok(files)
}

/// Announce the packages covered, so a run shows what it actually inspected.
fn announce(packages: &[String]) {
    eprintln!(
        "scan: {} package(s): {}",
        packages.len(),
        packages.join(", ")
    );
}

/// Everything one scan measured.
struct Measured {
    packages: Vec<String>,
    crates: Map<String, Value>,
    files_over_300: Vec<String>,
    functions_over_60: Vec<String>,
    unstable_features: Vec<String>,
    functions_over_logical: usize,
}

/// Measure every file, accumulating the per-package counts and the size-budget overruns.
fn measure(packages: &[Package], files: &[SourceFile]) -> Result<Measured> {
    let rules = Rules::compile()?;
    let mut scans: BTreeMap<String, CrateScan> = packages
        .iter()
        .map(|package| (package.name.clone(), CrateScan::new(&rules)))
        .collect();
    let mut measured = Measured {
        packages: packages
            .iter()
            .map(|package| package.name.clone())
            .collect(),
        crates: Map::new(),
        files_over_300: Vec::new(),
        functions_over_60: Vec::new(),
        unstable_features: Vec::new(),
        functions_over_logical: 0,
    };
    for file in files {
        let lines = read_lines(&file.path)?;
        let production = counts::production_lines(&lines, &rules);
        let label = file.label();
        let logical = counts::file_budgets(
            &label,
            &lines,
            &production,
            &rules,
            &mut measured.files_over_300,
            &mut measured.functions_over_60,
        );
        measured.functions_over_logical = measured.functions_over_logical.saturating_add(logical);
        let Some(scan) = scans.get_mut(&file.package) else {
            continue;
        };
        // A harness file counts against the package's file total but contributes no production
        // lines: it is a target the scan read, not source the budgets measure.
        scan.add_file(if file.harness { 0 } else { production.len() });
        if file.harness {
            continue;
        }
        for (line, name) in scan.add_production(&production, &rules) {
            measured
                .unstable_features
                .push(format!("{label}:{line} {name}"));
        }
    }
    for (name, scan) in scans {
        measured
            .crates
            .insert(name, Value::Object(scan.into_counts()));
    }
    measured.files_over_300.sort();
    measured.functions_over_60.sort();
    measured.unstable_features.sort();
    Ok(measured)
}

/// The whole report, as the JSON object the gate reads with `jq`.
fn report_value(measured: Measured) -> Value {
    let mut structure: Map<String, Value> = Map::new();
    structure.insert(
        "files_over_300_lines".to_string(),
        Value::Array(
            measured
                .files_over_300
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    structure.insert(
        "functions_over_60_lines".to_string(),
        Value::from(count(measured.functions_over_60.len())),
    );
    // The sites ride along as evidence: a budget the gate fails on has to name the function that
    // broke it, or the fix starts with a search instead of a read. They are not ratcheted — the count
    // above is the metric this scan certifies.
    structure.insert(
        "functions_over_60_sites".to_string(),
        Value::Array(
            measured
                .functions_over_60
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    structure.insert(
        "unstable_feature_sites".to_string(),
        Value::Array(
            measured
                .unstable_features
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    structure.insert(
        "functions_over_25_logical_lines".to_string(),
        Value::from(count(measured.functions_over_logical)),
    );
    let mut report: Map<String, Value> = Map::new();
    report.insert(
        "packages".to_string(),
        Value::Array(measured.packages.into_iter().map(Value::String).collect()),
    );
    report.insert("crates".to_string(), Value::Object(measured.crates));
    report.insert("structure".to_string(), Value::Object(structure));
    Value::Object(report)
}
