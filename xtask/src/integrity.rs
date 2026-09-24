//! Type-integrity scan for domain modules (Scott Wlaschin doctrine, measured not asserted).
//!
//! Counts candidates in the DOMAIN paths only:
//!
//! * `bool` parameters or return types in public signatures (boolean control flags);
//! * primitive id parameters (`String`/`&str`/integer named `*id`/`*_id`) where a newtype belongs;
//! * structs with two or more `Option<..>` fields (Option-as-state candidates).
//!
//! These are review candidates, not verdicts: each hit must be either converted or justified. The
//! counts are printed by `tools/gate.sh` and ratcheted in the DDD phase.
//!
//! Emits JSON on stdout: one object per domain root - one today, `census-domain` ->
//! `crates/census-domain/src` - keyed as the deleted `type_integrity_scan.py` keyed it, plus `ok`,
//! which is true only when every declared root produced at least one production file to measure.

use crate::json::count;
use crate::paths;
use crate::scan::{compile, is_test_file};
use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// The domain roots, keyed as the deleted `type_integrity_scan.py` keyed them.
const DOMAINS: [(&str, &str); 1] = [("census-domain", "crates/census-domain/src")];

/// A public function signature, up to its parameters and optional return type.
const PUB_FN: &str =
    r"^\s*pub\s+(?:async\s+)?fn\s+([a-zA-Z0-9_]+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)\s*(?:->\s*([^{]+))?";

/// A parameter whose type is a primitive named like an identifier.
const ID_PARAM: &str =
    r"\b([a-zA-Z0-9_]*id)\s*:\s*(?:&(?:'[a-z]+\s+)?)?(String|&str|u8|u16|u32|u64|usize|i32|i64)";

/// A `bool` in the signature: a parameter or a return type.
const BOOL_TOKEN: &str = r":\s*bool\b|->\s*bool\b";

/// A public struct declaration.
const PUB_STRUCT: &str = r"^\s*pub\s+struct\s+([A-Za-z0-9_]+)";

/// An `Option<..>` field inside a struct body.
const OPTION_FIELD: &str = r":\s*Option<";

/// Scan both domain roots and write the report to stdout.
///
/// The report carries `ok`, which is true only when every declared domain root produced at least one
/// production file. It is the difference between a scan of zero candidates and a scan of nothing:
/// `tools/gate.sh` fails the lane on `ok: false`, because a root that moved (or a filter that widened)
/// would otherwise read as a clean burndown.
pub fn run() -> Result<()> {
    let rules = Rules::compile()?;
    let root = paths::repo_root();
    let mut report: Map<String, Value> = Map::new();
    let mut healthy = true;
    for (name, relative_root) in DOMAINS {
        let mut hits = Hits::default();
        let mut files = 0usize;
        for path in domain_files(&root.join(relative_root))? {
            if is_test_file(&path) || !path.exists() {
                continue;
            }
            files = files.saturating_add(1);
            hits.measure(&path, &rules)?;
        }
        if files == 0 {
            healthy = false;
            eprintln!("integrity: {name} ({relative_root}) yielded no production file to measure");
        }
        report.insert(name.to_string(), Value::Object(hits.into_json()));
    }
    report.insert("ok".to_string(), Value::Bool(healthy));
    println!("{}", serde_json::to_string_pretty(&Value::Object(report))?);
    Ok(())
}

/// The candidate hits of one domain root, in scan order.
#[derive(Default)]
struct Hits {
    bool_in_signature: Vec<String>,
    primitive_id_param: Vec<String>,
    struct_with_many_options: Vec<String>,
}

impl Hits {
    /// Record the candidates one file carries.
    ///
    /// The deleted script cut the file at the first `#[cfg(test)]` line whatever it gated, and its
    /// struct walk started one line below the declaration, stopping at the first line holding a `}`.
    fn measure(&mut self, path: &Path, rules: &Rules) -> Result<()> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("reading {}", paths::relative(path)))?;
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let production = strip_test_cut(&lines);
        let file = file_name(path);
        for (offset, line) in production.iter().enumerate() {
            let number = offset.saturating_add(1);
            if let Some(captures) = rules.pub_fn.captures(line) {
                let name = captures.get(1).map_or("", |capture| capture.as_str());
                let params = captures.get(2).map_or("", |capture| capture.as_str());
                let returns = captures.get(3).map_or("", |capture| capture.as_str());
                if rules.bool_token.is_match(params) || rules.bool_token.is_match(returns) {
                    self.bool_in_signature
                        .push(format!("{file}:{number}:{name}"));
                }
                if let Some(id) = rules
                    .id_param
                    .captures(params)
                    .and_then(|captures| captures.get(1))
                {
                    self.primitive_id_param
                        .push(format!("{file}:{number}:{name}({})", id.as_str()));
                }
            }
            if rules.pub_struct.is_match(line) {
                let options = option_fields(production, number, rules);
                if options >= 2 {
                    self.struct_with_many_options
                        .push(format!("{file}:{number}:{options} option fields"));
                }
            }
        }
        Ok(())
    }

    fn into_json(self) -> Map<String, Value> {
        let mut classes: Map<String, Value> = Map::new();
        classes.insert(
            "bool_in_signature".to_string(),
            strings(self.bool_in_signature),
        );
        classes.insert(
            "primitive_id_param".to_string(),
            strings(self.primitive_id_param),
        );
        classes.insert(
            "struct_with_many_options".to_string(),
            strings(self.struct_with_many_options),
        );
        classes
    }
}

/// One class of hits as a JSON array of strings.
fn strings(hits: Vec<String>) -> Value {
    Value::Array(hits.into_iter().map(Value::String).collect())
}

/// `Option<..>` fields from the line after the declaration up to the first line holding a `}`.
fn option_fields(production: &[String], declaration: usize, rules: &Rules) -> u64 {
    let mut options = 0usize;
    let mut index = declaration;
    while let Some(line) = production.get(index) {
        if line.contains('}') {
            break;
        }
        if rules.option_field.is_match(line) {
            options = options.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    count(options)
}

/// Lines before the first `#[cfg(test)]` line, whatever it gates.
fn strip_test_cut(lines: &[String]) -> &[String] {
    let cut = lines
        .iter()
        .position(|line| line.trim() == "#[cfg(test)]")
        .unwrap_or(lines.len());
    lines.get(..cut).unwrap_or(lines)
}

/// The scanned files of one domain root: its `.rs` files, or the root itself when it is not a
/// directory.
fn domain_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.is_dir() {
        return Ok(vec![root.to_path_buf()]);
    }
    paths::rust_files(root)
}

/// The file name alone, which is how the report names a hit.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

/// Every pattern the scan matches, compiled once.
struct Rules {
    pub_fn: Regex,
    id_param: Regex,
    bool_token: Regex,
    pub_struct: Regex,
    option_field: Regex,
}

impl Rules {
    fn compile() -> Result<Self> {
        Ok(Self {
            pub_fn: compile(PUB_FN)?,
            id_param: compile(ID_PARAM)?,
            bool_token: compile(BOOL_TOKEN)?,
            pub_struct: compile(PUB_STRUCT)?,
            option_field: compile(OPTION_FIELD)?,
        })
    }
}
