//! `new-source`: generate a source adapter in the directory-module layout.
//!
//! The layout is the decomposition target: `mod.rs` (adapter entry point), `parse.rs` (pure), `map.rs`
//! (canonical mapping), a module README, and the fixture directory's README. The generated code
//! compiles and does nothing on purpose: `collect` bails, and `parse.rs` carries a fixture-driven
//! test that starts biting the moment a capture lands next to it.

use crate::paths;
use crate::templates::{adapter_module, adapter_readme, fixture_readme, map_module, parse_module};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Keywords that cannot name a module.
const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "union", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// One scaffold: where the module directory, the fixture directory and the colliding flat module
/// live.
struct Layout {
    name: String,
    adapter: PathBuf,
    fixtures: PathBuf,
    flat_module: PathBuf,
}

impl Layout {
    /// Resolve every path for `name`.
    fn resolve(name: String) -> Self {
        Self {
            adapter: paths::adapters_dir().join(&name),
            flat_module: paths::adapters_dir().join(format!("{name}.rs")),
            fixtures: paths::fixtures_dir().join(&name),
            name,
        }
    }

    /// Refuse before writing anything: an adapter directory, a flat module or a fixture directory of
    /// the same name already belongs to somebody.
    fn refuse(&self) -> Result<()> {
        for path in [&self.adapter, &self.flat_module, &self.fixtures] {
            if fs::symlink_metadata(path).is_ok() {
                bail!(
                    "refusing to scaffold `{}`: {} already exists; pick another name or move it by hand",
                    self.name,
                    paths::relative(path)
                );
            }
        }
        Ok(())
    }

    /// Every file the scaffold writes, with its body.
    fn files(&self) -> Vec<(PathBuf, String)> {
        let name = self.name.as_str();
        vec![
            (self.adapter.join("mod.rs"), adapter_module(name)),
            (self.adapter.join("parse.rs"), parse_module(name)),
            (self.adapter.join("map.rs"), map_module()),
            (self.adapter.join("README.md"), adapter_readme(name)),
            (self.fixtures.join("README.md"), fixture_readme(name)),
        ]
    }
}

/// Generate one adapter scaffold and register its module.
pub fn new_source(requested: &str) -> Result<()> {
    let name = module_name(requested)?;
    if name != requested {
        println!("normalized `{requested}` -> `{name}`: a module name is a Rust identifier");
    }
    let layout = Layout::resolve(name.clone());
    layout.refuse()?;
    let created = write_files(&layout.files())?;
    for path in &created {
        println!("created {}", paths::relative(path));
    }
    if let Err(error) = register_module(&name) {
        return Err(error.context(format!(
            "the {} file(s) listed above were created; add `pub mod {name};` to \
             crates/census-crawl/src/lib.rs by hand, or remove them",
            created.len()
        )));
    }
    println!(
        "\nnext: capture a fixture under {}, then run `cargo xtask source-test {name}`",
        paths::relative(&layout.fixtures)
    );
    Ok(())
}

/// The name as a module identifier: `probe-scaffold` and `probe_scaffold` both name `probe_scaffold`,
/// because the fixture directory and the nextest filter follow the module name.
fn module_name(requested: &str) -> Result<String> {
    let name = requested.trim().replace('-', "_");
    let starts_well = name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == '_');
    let rest_is_plain = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !starts_well || !rest_is_plain {
        bail!(
            "`{requested}` cannot name a source: use lowercase ASCII letters, digits and \
             underscores, starting with a letter (hyphens become underscores)"
        );
    }
    if KEYWORDS.contains(&name.as_str()) {
        bail!("`{name}` is a Rust keyword and cannot name a module");
    }
    Ok(name)
}

/// Write every file, stopping at the first failure and naming what already landed.
fn write_files(files: &[(PathBuf, String)]) -> Result<Vec<PathBuf>> {
    let mut created: Vec<PathBuf> = Vec::with_capacity(files.len());
    for (path, body) in files {
        if let Err(error) = write_one(path, body) {
            let so_far = created
                .iter()
                .map(|path| paths::relative(path))
                .collect::<Vec<String>>()
                .join(", ");
            let so_far = if so_far.is_empty() {
                "nothing".to_string()
            } else {
                so_far
            };
            return Err(error.context(format!("created so far: {so_far}")));
        }
        created.push(path.clone());
    }
    Ok(created)
}

/// Write one file, creating its directory first.
fn write_one(path: &Path, body: &str) -> Result<()> {
    let Some(parent) = path.parent() else {
        bail!("{} has no parent directory", paths::relative(path));
    };
    fs::create_dir_all(parent).with_context(|| format!("creating {}", paths::relative(parent)))?;
    fs::write(path, body).with_context(|| format!("writing {}", paths::relative(path)))
}

/// Append `pub mod <name>;` after the last declaration in the crawl crate's module list, unless it is
/// already declared. Returns whether the file changed.
fn register_module(name: &str) -> Result<bool> {
    let path = paths::adapters_dir().join("lib.rs");
    let text =
        fs::read_to_string(&path).with_context(|| format!("reading {}", paths::relative(&path)))?;
    let declaration = format!("pub mod {name};");
    if text.lines().any(|line| line.trim() == declaration) {
        println!(
            "{} already declares `{declaration}`",
            paths::relative(&path)
        );
        return Ok(false);
    }
    let lines: Vec<&str> = text.lines().collect();
    let Some(last) = lines
        .iter()
        .rposition(|line| line.trim_start().starts_with("pub mod "))
    else {
        bail!(
            "{} declares no `pub mod` line to insert after",
            paths::relative(&path)
        );
    };
    let mut updated = String::with_capacity(text.len().saturating_add(declaration.len()));
    for (index, line) in lines.iter().enumerate() {
        updated.push_str(line);
        updated.push('\n');
        if index == last {
            updated.push_str(&declaration);
            updated.push('\n');
        }
    }
    fs::write(&path, updated).with_context(|| format!("writing {}", paths::relative(&path)))?;
    println!("registered `{declaration}` in {}", paths::relative(&path));
    Ok(true)
}
