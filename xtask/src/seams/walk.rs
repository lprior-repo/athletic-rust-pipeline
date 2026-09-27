
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::paths;
use crate::scan::counts::{is_test_file, production_lines};
use crate::scan::rules::Rules;

use super::parse::refs_in_line;

pub(super) struct Reference {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) file: String,
    pub(super) line: usize,
}

pub(super) fn tree(src: &Path, rules: &Rules) -> Result<(BTreeSet<String>, Vec<Reference>)> {
    let mut modules = BTreeSet::new();
    let mut references = Vec::new();
    for file in paths::rust_files(src)? {
        collect(src, &file, rules, &mut modules, &mut references)?;
    }
    Ok((modules, references))
}

fn collect(
    src: &Path,
    file: &Path,
    rules: &Rules,
    modules: &mut BTreeSet<String>,
    references: &mut Vec<Reference>,
) -> Result<()> {
    if is_test_file(file) {
        return Ok(());
    }
    let Some(from) = top_module(src, file) else {
        return Ok(());
    };
    modules.insert(from.clone());
    let text =
        fs::read_to_string(file).with_context(|| format!("reading {}", paths::relative(file)))?;
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    for (index, line) in production_lines(&lines, rules).iter().enumerate() {
        for to in refs_in_line(line) {
            if to == from {
                continue;
            }
            references.push(Reference {
                from: from.clone(),
                to,
                file: paths::relative(file),
                line: index.saturating_add(1),
            });
        }
    }
    Ok(())
}

pub(super) fn top_module(src: &Path, file: &Path) -> Option<String> {
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
