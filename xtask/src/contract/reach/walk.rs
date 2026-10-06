use anyhow::Result;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::{declarations, masked_lines, module_dir, resolve, roots, Patterns};
use crate::paths;
use crate::scan::{read_lines, Member, Rules};

pub(super) struct Walk {
    pub(super) roots: usize,
    pub(super) reached: BTreeSet<PathBuf>,
    pub(super) failures: Vec<String>,
}

pub(super) fn run(
    members: &[Member],
    targets: &[PathBuf],
    rules: &Rules,
    patterns: &Patterns,
) -> Result<Walk> {
    let mut pending: Vec<(PathBuf, bool)> = roots(members, targets);
    let walk_roots = pending.len();
    let mut reached: BTreeSet<PathBuf> = BTreeSet::new();
    let mut failures: Vec<String> = Vec::new();
    while let Some((file, is_root)) = pending.pop() {
        if !reached.insert(file.clone()) {
            continue;
        }
        let visited = visit(&file, is_root, rules, patterns)?;
        pending.extend(visited.pending);
        failures.extend(visited.failures);
    }
    Ok(Walk {
        roots: walk_roots,
        reached,
        failures,
    })
}

pub(super) struct Visited {
    pub(super) pending: Vec<(PathBuf, bool)>,
    pub(super) failures: Vec<String>,
}

pub(super) fn visit(
    file: &Path,
    is_root: bool,
    rules: &Rules,
    patterns: &Patterns,
) -> Result<Visited> {
    let lines = read_lines(file)?;
    let masked = masked_lines(&lines, rules);
    let found = declarations(&masked, &lines, patterns);
    let mut visited = Visited {
        pending: Vec::new(),
        failures: Vec::new(),
    };
    let mut current = module_dir(file, is_root);
    let mut stack: Vec<PathBuf> = Vec::new();
    for (index, line) in masked.iter().enumerate() {
        track_directories(line, patterns, &mut current, &mut stack);
        for declaration in found.iter().filter(|found| found.line == index) {
            let candidates = resolve(file, &current, declaration);
            if candidates.is_empty() {
                visited.failures.push(format!(
                    "{} declares `mod {}`, which names no source file",
                    paths::relative(file),
                    declaration.name
                ));
                continue;
            }
            for candidate in candidates {
                visited.pending.push((candidate, false));
            }
        }
    }
    Ok(visited)
}

fn track_directories(
    line: &str,
    patterns: &Patterns,
    current: &mut PathBuf,
    stack: &mut Vec<PathBuf>,
) {
    let inline = patterns
        .inline
        .captures(line)
        .and_then(|captures| captures.get(1))
        .map(|name| name.as_str().to_string());
    let opens = line.matches('{').count();
    let closes = line.matches('}').count();
    let inline_braces = if inline.is_some() { 1 } else { 0 };
    if let Some(name) = inline {
        stack.push(current.clone());
        *current = current.join(name);
    }
    for _ in 0..opens.saturating_sub(inline_braces) {
        stack.push(current.clone());
    }
    for _ in 0..closes {
        if let Some(previous) = stack.pop() {
            *current = previous;
        }
    }
}
