use anyhow::Result;
use regex::Regex;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use super::Check;
use crate::cmd::Cmd;
use crate::paths;
use crate::scan::mask::CodeMask;
use crate::scan::{compile, members_in, targets_in, Member, Rules};

const NAME: &str = "source reachability";

const SKIP: [&str; 3] = ["target", ".git", "var"];

const DECLARATION: &str = r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;";

const INLINE: &str = r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{";

const PATH_ATTRIBUTE: &str = r"^\s*#\[path\s*=";

const PATH_LITERAL: &str = r#"^\s*#\[path\s*=\s*"([^"]*)""#;

struct Patterns {
    declaration: Regex,
    inline: Regex,
    path_attribute: Regex,
    path_literal: Regex,
}

impl Patterns {
    fn compile() -> Result<Self> {
        Ok(Self {
            declaration: compile(DECLARATION)?,
            inline: compile(INLINE)?,
            path_attribute: compile(PATH_ATTRIBUTE)?,
            path_literal: compile(PATH_LITERAL)?,
        })
    }
}

struct Declaration {
    line: usize,
    name: String,
    path: Option<String>,
}

pub(super) fn source_reachability() -> Result<Check> {
    let metadata = Cmd::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()?;
    let members = members_in(&metadata)?;
    let targets = targets_in(&metadata)?;
    let rules = Rules::compile()?;
    let patterns = Patterns::compile()?;
    let sources = source_files(&members)?;
    let walk = walk::run(&members, &targets, &rules, &patterns)?;
    let mut failures = walk.failures;
    if sources.is_empty() {
        failures.push(
            "the walk read no Rust file in any member package, so it cannot have found one"
                .to_string(),
        );
    }
    failures.extend(source_orphans(&sources, &walk.reached));
    let detail = format!(
        "{} target roots, {} Rust files across {} member packages, {} reached",
        walk.roots,
        sources.len(),
        members.len(),
        walk.reached.len()
    );
    if failures.is_empty() {
        return Ok(Check::holds(9, NAME, detail));
    }
    Ok(Check::violated(9, NAME, detail, failures))
}

fn source_files(members: &[Member]) -> Result<BTreeSet<PathBuf>> {
    let mut sources: BTreeSet<PathBuf> = BTreeSet::new();
    for member in members {
        for path in paths::rust_files_excluding(&member.dir, &SKIP)? {
            sources.insert(path);
        }
    }
    Ok(sources)
}

fn source_orphans(sources: &BTreeSet<PathBuf>, reached: &BTreeSet<PathBuf>) -> Vec<String> {
    sources
        .difference(reached)
        .map(|path| {
            format!(
                "{} is not a target root and no module declares it",
                paths::relative(path)
            )
        })
        .collect()
}

fn masked_lines(lines: &[String], rules: &Rules) -> Vec<String> {
    let mut mask = CodeMask::default();
    mask.apply_all(lines, &rules.char_literal)
}

fn declarations(masked: &[String], originals: &[String], patterns: &Patterns) -> Vec<Declaration> {
    let mut found: Vec<Declaration> = Vec::new();
    let mut declared_path: Option<String> = None;
    for (index, line) in masked.iter().enumerate() {
        let mut rest = line.as_str();
        if patterns.path_attribute.is_match(rest) {
            if let Some(original) = originals.get(index) {
                declared_path = patterns
                    .path_literal
                    .captures(original)
                    .and_then(|captures| captures.get(1))
                    .map(|literal| literal.as_str().to_string());
            }
            rest = rest
                .find(']')
                .and_then(|close| rest.get(close.saturating_add(1)..))
                .map_or("", std::convert::identity);
        }
        if let Some(captures) = patterns.declaration.captures(rest) {
            let name = captures
                .get(1)
                .map_or(Default::default(), |name| name.as_str().to_string());
            found.push(Declaration {
                line: index,
                name,
                path: declared_path.take(),
            });
            continue;
        }
        if rest.trim().is_empty() {
            continue;
        }
        declared_path = None;
    }
    found
}

fn module_dir(file: &Path, is_root: bool) -> PathBuf {
    let parent = file.parent().map_or(Default::default(), Path::to_path_buf);
    let named = file
        .file_name()
        .is_some_and(|name| name == OsStr::new("mod.rs"));
    if is_root || named {
        return parent;
    }
    let stem = file.file_stem().map_or(Default::default(), PathBuf::from);
    parent.join(stem)
}

fn resolve(declaring: &Path, directory: &Path, declaration: &Declaration) -> Vec<PathBuf> {
    let name = &declaration.name;
    let sibling = declaring
        .parent()
        .map_or(Default::default(), Path::to_path_buf);
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(relative) = &declaration.path {
        candidates.push(sibling.join(relative));
    } else {
        candidates.push(directory.join(format!("{name}.rs")));
        candidates.push(directory.join(name).join("mod.rs"));
        if sibling != directory {
            candidates.push(sibling.join(format!("{name}.rs")));
            candidates.push(sibling.join(name).join("mod.rs"));
        }
    }
    candidates
        .into_iter()
        .filter(|candidate| candidate.is_file())
        .map(|candidate| normalize(&candidate))
        .collect()
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                if !normalized.pop() && !normalized.has_root() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn roots(members: &[Member], targets: &[PathBuf]) -> Vec<(PathBuf, bool)> {
    let mut roots: Vec<(PathBuf, bool)> = targets
        .iter()
        .filter(|target| target.is_file())
        .map(|target| (normalize(target), true))
        .collect();
    for member in members {
        let script = member.dir.join("build.rs");
        if script.is_file() {
            roots.push((script, true));
        }
    }
    roots
}

mod walk;

#[cfg(test)]
#[path = "reach/tests.rs"]
mod tests;
