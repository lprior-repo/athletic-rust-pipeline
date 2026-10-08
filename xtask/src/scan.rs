pub(crate) mod counts;
pub(crate) mod mask;
mod packages;
pub(crate) mod rules;
pub(crate) mod strict;

use crate::paths;
use anyhow::{Context, Result};
use counts::CrateScan;
use packages::{Package, Root};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) use counts::is_test_file;
pub(crate) use packages::{members, members_in, targets_in, Member};
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
    let report = report_value(measured);
    println!("{}", serde_json::to_string_pretty(&report)?);
    strict::enforce_report(&report)
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
    Ok(all_root_files(package, root)?
        .into_iter()
        .filter(|file| !counts::is_test_file(&file.path))
        .collect())
}

fn all_root_files(package: &str, root: &Root) -> Result<Vec<SourceFile>> {
    let paths = if root.path.is_file() {
        vec![root.path.clone()]
    } else {
        paths::rust_files(&root.path)?
    };
    let mut files = Vec::new();
    for path in paths {
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
    strict: strict::Findings,
    unstable_features: Vec<String>,
}

fn measure(packages: &[Package], files: &[SourceFile]) -> Result<Measured> {
    let rules = Rules::compile()?;
    let mut scans: BTreeMap<String, CrateScan> = packages
        .iter()
        .map(|package| (package.name.clone(), CrateScan::new(&rules)))
        .collect();
    let mut measured = Measured::new(packages);
    for file in files {
        measure_file(file, &rules, &mut scans, &mut measured)?;
    }
    measured.strict = strict_measure(packages)?;
    for (name, scan) in scans {
        measured
            .crates
            .insert(name, Value::Object(scan.into_counts()));
    }
    measured.files_over_300.sort();
    measured.unstable_features.sort();
    Ok(measured)
}

impl Measured {
    fn new(packages: &[Package]) -> Self {
        Self {
            packages: packages
                .iter()
                .map(|package| package.name.clone())
                .collect(),
            crates: Map::new(),
            files_over_300: Vec::new(),
            strict: strict::Findings::default(),
            unstable_features: Vec::new(),
        }
    }
}

fn measure_file(
    file: &SourceFile,
    rules: &Rules,
    scans: &mut BTreeMap<String, CrateScan>,
    measured: &mut Measured,
) -> Result<()> {
    let lines = read_lines(&file.path)?;
    let production = counts::production_lines(&lines, rules);
    let label = file.label();
    counts::file_budgets(&label, &lines, &mut measured.files_over_300);
    let scan = scans
        .get_mut(&file.package)
        .context("source belongs to unmeasured package")?;
    scan.add_file(if file.harness { 0 } else { production.len() });
    if file.harness {
        return Ok(());
    }
    for (line, name) in scan.add_production(&production, rules) {
        measured
            .unstable_features
            .push(format!("{label}:{line} {name}"));
    }
    Ok(())
}

fn strict_measure(packages: &[Package]) -> Result<strict::Findings> {
    let files = strict_files(packages)?;
    let roots = packages::production_targets()?;
    let sources = strict_sources(&files, &roots)?;
    let tests = strict::modules::test_files(&sources, &roots)?;
    strict::inspect_sources(&sources, &roots, &tests)
}

fn strict_files(packages: &[Package]) -> Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    for package in packages {
        for root in package.roots.iter().filter(|root| !root.harness) {
            files.extend(all_root_files(&package.name, root)?);
        }
    }
    Ok(files)
}

fn strict_sources(files: &[SourceFile], roots: &[PathBuf]) -> Result<Vec<(PathBuf, String)>> {
    let mut pending = Vec::new();
    let mut admitted = std::collections::BTreeSet::new();
    for path in files.iter().map(|file| &file.path).chain(roots) {
        admit_source(path.clone(), &mut pending, &mut admitted)?;
    }
    let mut sources = Vec::new();
    for _ in 0..100_000 {
        let Some(path) = pending.pop() else {
            return Ok(sources);
        };
        let source = fs::read_to_string(&path).with_context(|| paths::relative(&path))?;
        for target in strict::modules::references(&path, &source, roots)? {
            admit_source(target, &mut pending, &mut admitted)?;
        }
        sources.try_reserve(1)?;
        sources.push((path, source));
    }
    anyhow::ensure!(
        pending.is_empty(),
        "strict source traversal budget exhausted"
    );
    Ok(sources)
}

fn admit_source(
    path: PathBuf,
    pending: &mut Vec<PathBuf>,
    admitted: &mut std::collections::BTreeSet<PathBuf>,
) -> Result<()> {
    if admitted.contains(&path) {
        return Ok(());
    }
    anyhow::ensure!(
        admitted.len() < 100_000,
        "strict source file budget exhausted"
    );
    pending.try_reserve(1)?;
    admitted.insert(path.clone());
    pending.push(path);
    Ok(())
}

fn report_value(measured: Measured) -> Value {
    let mut structure = Map::new();
    structure.insert(
        "files_over_300_lines".to_string(),
        string_array(measured.files_over_300),
    );
    structure.insert(
        "unstable_feature_sites".to_string(),
        string_array(measured.unstable_features),
    );
    measured.strict.insert(&mut structure);
    let mut report = Map::new();
    report.insert("packages".to_string(), string_array(measured.packages));
    report.insert("crates".to_string(), Value::Object(measured.crates));
    report.insert("structure".to_string(), Value::Object(structure));
    Value::Object(report)
}

fn string_array(items: Vec<String>) -> Value {
    Value::Array(items.into_iter().map(Value::String).collect())
}
