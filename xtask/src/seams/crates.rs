//! The crate graph: which workspace crate may name which, and the walk that answers it.
//!
//! A module seam is sealed by the compiler — a private submodule cannot be reached from a sibling —
//! and a crate seam is not: `census-review` can name `census-store` in a manifest and nothing but a
//! measurement keeps the direction the architecture declares. Splitting the census into crates
//! therefore moves seams out of [`super::ALLOWED`] and into [`super::ALLOWED_CRATES`], and this module
//! is the measurer for the second table. It reads the workspace's own package graph from
//! `cargo metadata --no-deps` (offline: names, manifest paths, declared dependencies), walks every
//! package's production source for identifiers that resolve to a sibling workspace crate, and reports
//! each `(from, to)` pair with the file and line it sits on.
//!
//! Only *workspace* crates are edges. `serde`, `reqwest` and the rest are dependencies, not seams:
//! this table describes the census's own shape, exactly as the module table does. Aliases are read
//! from the manifest rather than guessed from a crate name — a renamed dependency contributes the
//! identifier the code actually writes — and dev and build dependencies are not production edges, so
//! a crate that appears only under `[dev-dependencies]` contributes no alias.
//!
//! The same production judgement as [`super::walk`] applies: `is_test_file` and `production_lines`
//! come from the scanner, so a test-only reference is not an edge here either.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::paths;
use crate::scan::counts::{is_test_file, production_lines};
use crate::scan::rules::Rules;

use super::parse::crates_in_line;
use super::walk::Reference;

/// One workspace package: the name the crate table uses, its source tree, and the aliases its own
/// manifest gives its siblings.
pub(super) struct Package {
    pub(super) name: String,
    src: PathBuf,
    aliases: BTreeMap<String, String>,
}

/// Every workspace package, each with its `alias -> package` map for the siblings it declares.
pub(super) fn packages() -> Result<Vec<Package>> {
    let metadata = metadata()?;
    let listed = metadata
        .get("packages")
        .and_then(Value::as_array)
        .context("`cargo metadata` reported no `packages`")?;
    let members: BTreeSet<String> = listed
        .iter()
        .filter_map(|package| package.get("name").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    let mut workspace = Vec::new();
    for package in listed {
        let Some(name) = package.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(manifest) = package.get("manifest_path").and_then(Value::as_str) else {
            continue;
        };
        let root = Path::new(manifest)
            .parent()
            .with_context(|| format!("the manifest path of `{name}` has no directory"))?;
        workspace.push(Package {
            name: name.to_string(),
            src: root.join("src"),
            aliases: aliases_of(package, &members),
        });
    }
    Ok(workspace)
}

/// The sibling crates one package's manifest lets its code name, as `alias -> package`.
fn aliases_of(package: &Value, members: &BTreeSet<String>) -> BTreeMap<String, String> {
    let mut aliases = BTreeMap::new();
    let dependencies = package
        .get("dependencies")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for dependency in dependencies {
        if dependency.get("kind").and_then(Value::as_str).is_some() {
            continue;
        }
        let Some(dep_name) = dependency.get("name").and_then(Value::as_str) else {
            continue;
        };
        if !members.contains(dep_name) {
            continue;
        }
        let alias = dependency
            .get("rename")
            .and_then(Value::as_str)
            .unwrap_or(dep_name)
            .replace('-', "_");
        aliases.insert(alias, dep_name.to_string());
    }
    aliases
}

/// The workspace package graph as `cargo metadata --no-deps --format-version 1` reports it.
fn metadata() -> Result<Value> {
    let output = Command::new("cargo")
        .current_dir(paths::repo_root())
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .context("running `cargo metadata --no-deps --format-version 1`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`cargo metadata --no-deps` failed: {}", stderr.trim());
    }
    serde_json::from_slice(&output.stdout).context("parsing the `cargo metadata` report")
}

/// Every workspace crate and every cross-crate reference in its production source.
pub(super) fn tree(
    packages: &[Package],
    rules: &Rules,
) -> Result<(BTreeSet<String>, Vec<Reference>)> {
    let mut crates = BTreeSet::new();
    let mut references = Vec::new();
    for package in packages {
        crates.insert(package.name.clone());
        if !package.src.is_dir() {
            continue;
        }
        for file in paths::rust_files(&package.src)? {
            collect(package, &file, rules, &mut references)?;
        }
    }
    Ok((crates, references))
}

/// One file's contribution: every sibling crate its production region names.
fn collect(
    package: &Package,
    file: &Path,
    rules: &Rules,
    references: &mut Vec<Reference>,
) -> Result<()> {
    if is_test_file(file) {
        return Ok(());
    }
    let text =
        fs::read_to_string(file).with_context(|| format!("reading {}", paths::relative(file)))?;
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    for (index, line) in production_lines(&lines, rules).iter().enumerate() {
        for to in crates_in_line(line, &package.aliases) {
            if to == package.name {
                continue;
            }
            references.push(Reference {
                from: package.name.clone(),
                to,
                file: paths::relative(file),
                line: index.saturating_add(1),
            });
        }
    }
    Ok(())
}
