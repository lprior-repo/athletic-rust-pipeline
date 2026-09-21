//! Domain-crate purity proof: the `census-domain` dependency tree carries no async runtime, store
//! engine, HTTP client, service framework or browser engine.
//!
//! This is the enforceable form of the Phase 3 acceptance criterion ("ban proof that the domain
//! crate's tree has zero async/I/O deps"). cargo-deny's `wrappers` bans express the inverse relation
//! (a banned crate allowed only under a listed wrapper), so the tree itself is the evidence here:
//! normal edges only, which excludes dev-dependencies and build scripts, so test-only crates cannot
//! taint the verdict either way.

use crate::paths;
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::process::Command;

/// The tree command this proof reads, spelled once so the error message cannot drift from it.
const TREE: [&str; 7] = [
    "tree",
    "-p",
    "census-domain",
    "--edges",
    "normal",
    "--prefix",
    "none",
];

/// Packages that must not appear in the domain crate's normal dependency tree.
const BANNED: [&str; 24] = [
    "tokio",
    "fjall",
    "reqwest",
    "serde_json",
    "restate-sdk",
    "restate-sdk-shared-core",
    "chromiumoxide",
    "chromiumoxide-cdp",
    "chromiumoxide_types",
    "hyper",
    "hyper-util",
    "axum",
    "axum-core",
    "tower",
    "tower-http",
    "mio",
    "h2",
    "rustls",
    "openssl",
    "socket2",
    "tungstenite",
    "async-tungstenite",
    "tokio-util",
    "tokio-rustls",
];

/// Resolve the tree, print it, and fail when a banned package is in it.
pub fn run() -> Result<()> {
    let output = Command::new("cargo")
        .args(TREE)
        .current_dir(paths::repo_root())
        .output()
        .context("running `cargo tree -p census-domain --edges normal --prefix none`")?;
    if !output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stderr).trim_end());
        println!();
        bail!("domain purity: could not resolve the census-domain tree");
    }
    let tree = String::from_utf8_lossy(&output.stdout);
    let packages = packages(&tree);
    println!(
        "  census-domain normal tree: {}",
        packages.iter().copied().collect::<Vec<&str>>().join(", ")
    );
    let violations: Vec<&str> = packages
        .iter()
        .copied()
        .filter(|name| BANNED.contains(name))
        .collect();
    if !violations.is_empty() {
        bail!("FORBIDDEN dependencies present: {}", violations.join(", "));
    }
    println!("  no async/I-O dependency present");
    Ok(())
}

/// The distinct package names of a `cargo tree --prefix none` listing, sorted: each line starts with
/// the package name, followed by its version and source.
fn packages(tree: &str) -> BTreeSet<&str> {
    tree.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .collect()
}
