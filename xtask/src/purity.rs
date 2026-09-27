use crate::paths;
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::process::Command;

const TREE: [&str; 7] = [
    "tree",
    "-p",
    "census-domain",
    "--edges",
    "normal",
    "--prefix",
    "none",
];

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

fn packages(tree: &str) -> BTreeSet<&str> {
    tree.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .collect()
}
