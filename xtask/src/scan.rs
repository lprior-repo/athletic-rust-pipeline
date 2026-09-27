
pub(crate) mod counts;
mod mask;
mod packages;
pub(crate) mod rules;

use crate::json::count;
use crate::paths;
use anyhow::{Context, Result};
use counts::CrateScan;
use packages::{Package, Root};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) use counts::is_test_file;
pub(crate) use packages::members;
pub(crate) use rules::{compile, Rules};

pub(crate) struct SourceFile {
    pub(crate) package: String,
    pub(crate) path: PathBuf,
    pub(crate) harness: bool,
}

impl SourceFile {
    pub(crate) fn label(&self) -> String {
        format!("{}:{}", self.package, paths::relative(&self.path))
    }
}

pub(crate) fn run() -> Result<()> {
    let (packages, files) = walk()?;
    let measured = measure(&packages, &files)?;
    announce(&measured.packages);
    println!("{}", serde_json::to_string_pretty(&report_value(measured))?);
    Ok(())
}

pub(crate) fn report() -> Result<Value> {
    let (packages, files) = walk()?;
    Ok(report_value(measure(&packages, &files)?))
}

pub(crate) fn source_files() -> Result<Vec<SourceFile>> {
    Ok(walk()?.1)
}

pub(crate) fn masked_production(file: &SourceFile, rules: &Rules) -> Result<Vec<String>> {
    let lines = read_lines(&file.path)?;
    let production = counts::production_lines(&lines, rules);
    let mut mask = mask::CodeMask::default();
    Ok(mask.apply_all(&production, &rules.char_literal))
}

pub(crate) fn read_lines(path: &Path) -> Result<Vec<String>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading {}", paths::relative(path)))?;
    Ok(text.lines().map(str::to_string).collect())
}

fn walk() -> Result<(Vec<Package>, Vec<SourceFile>)> {
    let packages = packages::scanned()?;
    let mut files = Vec::new();
    for package in &packages {
        for root in &package.roots {
            files.extend(root_files(&package.name, root)?);
        }
    }
    Ok((packages, files))
}

fn root_files(package: &str, root: &Root) -> Result<Vec<SourceFile>> {
    let paths = if root.path.is_file() {
        vec![root.path.clone()]
    } else {
        paths::rust_files(&root.path)?
    };
    let mut files = Vec::new();
    for path in paths {
        if counts::is_test_file(&path) {
            continue;
        }
        files.push(SourceFile {
            package: package.to_string(),
            path,
            harness: root.harness,
        });
    }
    Ok(files)
}

fn announce(packages: &[String]) {
    eprintln!(
        "scan: {} package(s): {}",
        packages.len(),
        packages.join(", ")
    );
}

struct Measured {
    packages: Vec<String>,
    crates: Map<String, Value>,
    files_over_300: Vec<String>,
    functions_over_60: Vec<String>,
    unstable_features: Vec<String>,
    functions_over_logical: usize,
}

fn measure(packages: &[Package], files: &[SourceFile]) -> Result<Measured> {
    let rules = Rules::compile()?;
    let mut scans: BTreeMap<String, CrateScan> = packages
        .iter()
        .map(|package| (package.name.clone(), CrateScan::new(&rules)))
        .collect();
    let mut measured = Measured {
        packages: packages
            .iter()
            .map(|package| package.name.clone())
            .collect(),
        crates: Map::new(),
        files_over_300: Vec::new(),
        functions_over_60: Vec::new(),
        unstable_features: Vec::new(),
        functions_over_logical: 0,
    };
    for file in files {
        let lines = read_lines(&file.path)?;
        let production = counts::production_lines(&lines, &rules);
        let label = file.label();
        let logical = counts::file_budgets(
            &label,
            &lines,
            &production,
            &rules,
            &mut measured.files_over_300,
            &mut measured.functions_over_60,
        );
        measured.functions_over_logical = measured.functions_over_logical.saturating_add(logical);
        let Some(scan) = scans.get_mut(&file.package) else {
            continue;
        };
        scan.add_file(if file.harness { 0 } else { production.len() });
        if file.harness {
            continue;
        }
        for (line, name) in scan.add_production(&production, &rules) {
            measured
                .unstable_features
                .push(format!("{label}:{line} {name}"));
        }
    }
    for (name, scan) in scans {
        measured
            .crates
            .insert(name, Value::Object(scan.into_counts()));
    }
    measured.files_over_300.sort();
    measured.functions_over_60.sort();
    measured.unstable_features.sort();
    Ok(measured)
}

fn report_value(measured: Measured) -> Value {
    let mut structure: Map<String, Value> = Map::new();
    structure.insert(
        "files_over_300_lines".to_string(),
        Value::Array(
            measured
                .files_over_300
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    structure.insert(
        "functions_over_60_lines".to_string(),
        Value::from(count(measured.functions_over_60.len())),
    );
    structure.insert(
        "functions_over_60_sites".to_string(),
        Value::Array(
            measured
                .functions_over_60
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    structure.insert(
        "unstable_feature_sites".to_string(),
        Value::Array(
            measured
                .unstable_features
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    structure.insert(
        "functions_over_25_logical_lines".to_string(),
        Value::from(count(measured.functions_over_logical)),
    );
    let mut report: Map<String, Value> = Map::new();
    report.insert(
        "packages".to_string(),
        Value::Array(measured.packages.into_iter().map(Value::String).collect()),
    );
    report.insert("crates".to_string(), Value::Object(measured.crates));
    report.insert("structure".to_string(), Value::Object(structure));
    Value::Object(report)
}
