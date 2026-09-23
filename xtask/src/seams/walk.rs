//! Which files the seam check reads, and which top-level module each of them belongs to.
//!
//! Two judgements decide the answer, and neither is made here: `is_test_file` and `production_lines`
//! come from the scanner, because a file that is test code or a region that sits behind
//! `#[cfg(test)] mod tests` is exactly what the size budgets also exclude. Asking the scanner rather
//! than re-deriving it is what keeps one definition of "production" in the tree — a second copy of
//! this walk would let the seam check and the budgets disagree about a file without either failing.
//!
//! `main.rs`, `cli/**` and `bin/**` are out of scope on top of that: every binary is its own crate
//! root, so those files consume the library through its public API (`midwest_census::…`) and cannot
//! form a library-internal seam.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::paths;
use crate::scan::counts::{is_test_file, production_lines};
use crate::scan::rules::Rules;

use super::parse::refs_in_line;

/// One resolved reference: source module, target top module, and where it sits.
pub(super) struct Reference {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) file: String,
    pub(super) line: usize,
}

/// Every top-level module and every `crate::…` reference in the census production source.
pub(super) fn tree(src: &Path, rules: &Rules) -> Result<(BTreeSet<String>, Vec<Reference>)> {
    let mut modules = BTreeSet::new();
    let mut references = Vec::new();
    for file in paths::rust_files(src)? {
        collect(src, &file, rules, &mut modules, &mut references)?;
    }
    Ok((modules, references))
}

/// One file's contribution: the module it belongs to, and the references its production region
/// makes. A reference to the file's own module is not a seam and is dropped where it is found.
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

/// The top-level module a source file belongs to, `None` for bin-tree files.
///
/// `src/store/read.rs` and `src/store/mod.rs` are both `store`; a file directly under `src/`
/// (`lib.rs`, `bootstrap.rs`) is its own stem. `main.rs`, `cli/**` and `bin/**` are excluded:
/// every binary is its own crate root, so those files consume the library through its public
/// API (`midwest_census::…`) and cannot form a lib-internal seam.
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
