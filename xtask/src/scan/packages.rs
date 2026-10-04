use crate::cmd::Cmd;
use crate::paths;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

const ROOTS: [(&str, bool); 3] = [
    ("src", false),
    ("examples", true),
    ("benches", true),
];

const FUZZ_TARGETS: [&str; 2] = ["fuzz", "fuzz_targets"];

pub(crate) struct Member {
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
}

pub(crate) struct Root {
    pub(crate) path: PathBuf,
    pub(crate) harness: bool,
}

pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) roots: Vec<Root>,
}

pub(crate) fn members() -> Result<Vec<Member>> {
    let metadata = Cmd::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()?;
    members_in(&metadata)
}

pub(crate) fn scanned() -> Result<Vec<Package>> {
    let root = paths::repo_root();
    Ok(first_party(&members()?, &root))
}

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

fn build_script(dir: &Path) -> Option<Root> {
    let path = dir.join("build.rs");
    path.is_file().then_some(Root {
        path,
        harness: false,
    })
}

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
