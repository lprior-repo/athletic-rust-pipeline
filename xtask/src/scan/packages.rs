//! The package set the scan covers, enumerated from `cargo metadata` rather than kept by hand.
//!
//! The scan used to name its packages in a two-row table. That table is why three of the workspace's
//! then-members — `census-domain`, `g1-audit` and `xtask` itself — escaped every construct count and
//! every size budget: widening a scan is supposed to be a deliberate act, but a hand-kept list makes
//! *joining the workspace* the act, and nobody notices a missing row until someone ports the scanner
//! by hand and compares.
//!
//! `cargo metadata --no-deps --format-version 1` answers with the workspace members and nothing else
//! (`--no-deps` drops the dependency graph; the vendors and the separate `fuzz/` workspace are not
//! members), so the package set is the workspace's, and `xtask contract` asserts that equality from
//! the other side: a member that reaches the workspace without reaching the scan fails the contract
//! verb.

use crate::cmd::Cmd;
use crate::paths;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The Rust source roots of one package, relative to its manifest: the directory, and whether the
/// files it holds compile into a production target.
///
/// `src/` is what a library or binary target is built from, so it is production. `examples/`,
/// `benches/` and `kani/` are harnesses by construction: an example target is built only when someone
/// asks for it (two of the three here are manual benchmarks, the third a fixture server), a benchmark
/// target has no error channel (`benches/*.rs` aborts the run when its corpus will not build) and a
/// Kani harness asserts by definition. None of the three is reachable from a production build, so
/// their constructs are not production debt — their *size* still is, because a 500-line example is
/// a file that is too long.
/// The gate's strict clippy lane keeps analyzing them (`--lib --bins --examples` names them as source
/// targets rather than test targets), which is where a lint can see the whole target.
const ROOTS: [(&str, bool); 4] = [
    ("src", false),
    ("examples", true),
    ("benches", true),
    ("kani", true),
];

/// The cargo-fuzz target directory, relative to a package's manifest.
const FUZZ_TARGETS: [&str; 2] = ["fuzz", "fuzz_targets"];

/// One workspace member as `cargo metadata --no-deps` reports it: its package name and the directory
/// holding its manifest.
pub(crate) struct Member {
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
}

/// One package's production root: the directory to walk and whether its files are harness code.
pub(crate) struct Root {
    pub(crate) path: PathBuf,
    pub(crate) harness: bool,
}

/// One first-party package: the name the report and the debt baseline key it by, the directory its
/// manifest sits in, and the roots to walk.
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) roots: Vec<Root>,
}

/// Every workspace member, in name order.
pub(crate) fn members() -> Result<Vec<Member>> {
    let metadata = Cmd::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()?;
    members_in(&metadata)
}

/// The first-party packages the scan covers: every member whose manifest sits inside the repository.
///
/// The filter is about provenance rather than membership. `cargo metadata --no-deps` lists members
/// only, so a package it reports is in this workspace; one whose manifest lies outside the repository
/// is not this repository's code to hold to this repository's policy, and `xtask contract` names it
/// rather than letting it join the scan silently.
pub(crate) fn scanned() -> Result<Vec<Package>> {
    let root = paths::repo_root();
    Ok(first_party(&members()?, &root))
}

/// The members of a `cargo metadata --no-deps` document, sorted by name.
///
/// Split from [`members`] so the parsing has no process behind it: the shape of the document is
/// `cargo`'s, and a test can hand this function the JSON it needs without running the toolchain.
pub(crate) fn members_in(metadata: &str) -> Result<Vec<Member>> {
    let document: Value =
        serde_json::from_str(metadata).context("parsing `cargo metadata` output as JSON")?;
    let listed = document
        .get("packages")
        .and_then(Value::as_array)
        .context("`cargo metadata` output has no `packages` array")?;
    let mut members = Vec::with_capacity(listed.len());
    for entry in listed {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .context("a `cargo metadata` package has no name")?;
        let manifest = entry
            .get("manifest_path")
            .and_then(Value::as_str)
            .context("a `cargo metadata` package has no manifest path")?;
        let dir = Path::new(manifest)
            .parent()
            .context("a `cargo metadata` manifest path has no directory")?;
        members.push(Member {
            name: name.to_string(),
            dir: dir.to_path_buf(),
        });
    }
    members.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(members)
}

/// The members under `root`, with their roots enumerated.
pub(crate) fn first_party(members: &[Member], root: &Path) -> Vec<Package> {
    members
        .iter()
        .filter(|member| member.dir.starts_with(root))
        .map(|member| Package {
            name: member.name.clone(),
            roots: roots(&member.dir),
        })
        .collect()
}

/// The roots of one package that exist on disk, in [`ROOTS`] order, plus its build script and its
/// cargo-fuzz targets.
fn roots(dir: &Path) -> Vec<Root> {
    let named = ROOTS.iter().filter_map(|(name, harness)| {
        let path = dir.join(name);
        path.is_dir().then_some(Root {
            path,
            harness: *harness,
        })
    });
    let mut roots: Vec<Root> = named.collect();
    roots.extend(build_script(dir));
    roots.extend(fuzz_targets(dir));
    roots
}

/// A package's build script, when it carries one.
///
/// Cargo compiles `build.rs` into the build, so it is production code rather than a harness — and it
/// is a *file*, which is why [`ROOTS`] cannot name it: every other root is a directory to walk. No
/// member of this workspace ships one today, so this rule measures nothing yet; it is here so the
/// first one that appears is measured rather than joining the blind spot the walker exists to close.
fn build_script(dir: &Path) -> Option<Root> {
    let path = dir.join("build.rs");
    path.is_file().then_some(Root {
        path,
        harness: false,
    })
}

/// A package's cargo-fuzz targets, when it carries its own rather than a workspace of its own.
///
/// `fuzz/fuzz_targets` is a convention, and a `fuzz/Cargo.toml` beside it is the other half of it:
/// cargo-fuzz writes that manifest because the targets are their own package, compiled against its
/// own dependency versions and its own `[workspace]`. When it is there the directory belongs to
/// another package — the repository root carries exactly such a workspace — so this scan does not
/// walk it: this policy is enforced on this workspace's production code, and a fuzz target is neither
/// production code nor this package's file.
fn fuzz_targets(dir: &Path) -> Option<Root> {
    let fuzz = dir.join(FUZZ_TARGETS.first()?);
    let targets = dir.join(FUZZ_TARGETS.join("/"));
    if !targets.is_dir() || fuzz.join("Cargo.toml").is_file() {
        return None;
    }
    Some(Root {
        path: targets,
        harness: true,
    })
}

#[cfg(test)]
#[path = "packages/tests.rs"]
mod tests;
