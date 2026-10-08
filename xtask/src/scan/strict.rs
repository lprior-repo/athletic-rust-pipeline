mod argument_tokens;
mod arguments;
mod attributes;
mod callables;
mod exclusions;
mod graph;
mod imports;
mod measure;
pub(crate) mod modules;
mod obligations;
mod provenance;
mod syntax;
mod templates;
#[cfg(test)]
mod tests;

use anyhow::{ensure, Context, Result};
use serde_json::{Map, Value};

const POLICY: &str = "ast-handwritten-25-advisory-60-max-5-review-v3";

#[derive(Default, Debug)]
pub(crate) struct Findings {
    pub(crate) lines: Vec<String>,
    pub(crate) over_60: Vec<String>,
    pub(crate) parameters: Vec<String>,
    pub(crate) unresolved: Vec<String>,
    pub(crate) callables: usize,
    pub(crate) files: usize,
}

#[cfg(test)]
pub(crate) fn inspect(source: &str) -> Result<Findings> {
    let path = std::path::PathBuf::from("/strict-source.rs");
    inspect_sources(
        &[(path.clone(), source.to_string())],
        &[path],
        &std::collections::BTreeSet::new(),
    )
}

pub(crate) fn inspect_sources(
    sources: &[(std::path::PathBuf, String)],
    roots: &[std::path::PathBuf],
    tests: &std::collections::BTreeSet<std::path::PathBuf>,
) -> Result<Findings> {
    let parsed = sources
        .iter()
        .map(|(path, source)| {
            Ok(imports::Parsed {
                path,
                source,
                file: syn::parse_file(source).with_context(|| {
                    format!("strict Rust AST parsing failed: {}", path.display())
                })?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut catalog = imports::Catalog::collect(&parsed, roots)?;
    let mut found = Findings::default();
    for source in parsed.iter().filter(|source| !tests.contains(source.path)) {
        let templates = catalog.take_templates(source.path);
        let arguments = catalog.take_arguments(source.path);
        let view = catalog.view(source.path)?;
        let measured = callables::inspect(&source.file, source.source, view, templates, arguments)?;
        found.append(&crate::paths::relative(source.path), measured)?;
    }
    Ok(found)
}

impl Findings {
    pub(crate) fn append(&mut self, label: &str, other: Self) -> Result<()> {
        self.files = self
            .files
            .checked_add(other.files)
            .context("strict file count overflow")?;
        self.callables = self
            .callables
            .checked_add(other.callables)
            .context("strict callable count overflow")?;
        self.lines.extend(
            other
                .lines
                .into_iter()
                .map(|site| format!("{label}:{site}")),
        );
        self.over_60.extend(
            other
                .over_60
                .into_iter()
                .map(|site| format!("{label}:{site}")),
        );
        self.parameters.extend(
            other
                .parameters
                .into_iter()
                .map(|site| format!("{label}:{site}")),
        );
        self.unresolved.extend(
            other
                .unresolved
                .into_iter()
                .map(|site| format!("{label}:{site}")),
        );
        Ok(())
    }

    pub(crate) fn insert(self, structure: &mut Map<String, Value>) {
        structure.insert("strict_policy".to_string(), Value::from(POLICY));
        structure.insert("strict_files_checked".to_string(), Value::from(self.files));
        structure.insert(
            "strict_callables_checked".to_string(),
            Value::from(self.callables),
        );
        structure.insert(
            "functions_over_60_lines".to_string(),
            Value::from(self.over_60.len()),
        );
        structure.insert("functions_over_60_sites".to_string(), strings(self.over_60));
        structure.insert(
            "functions_over_25_logical_lines".to_string(),
            Value::from(self.lines.len()),
        );
        structure.insert(
            "functions_over_5_parameters".to_string(),
            Value::from(self.parameters.len()),
        );
        structure.insert("functions_over_25_sites".to_string(), strings(self.lines));
        structure.insert(
            "functions_over_5_parameter_sites".to_string(),
            strings(self.parameters),
        );
        structure.insert(
            "strict_unresolved_sites".to_string(),
            strings(self.unresolved),
        );
    }
}

fn strings(sites: Vec<String>) -> Value {
    Value::Array(sites.into_iter().map(Value::String).collect())
}

pub(crate) fn enforce_report(report: &Value) -> Result<()> {
    validate_report(report)?;
    let current = crate::scan::report().context("remeasuring current strict AST evidence")?;
    let supplied = report
        .get("structure")
        .context("strict structure missing")?;
    let measured = current
        .get("structure")
        .context("current strict structure missing")?;
    for key in STRICT_FIELDS {
        ensure!(
            supplied.get(*key) == measured.get(*key),
            "strict evidence is stale or forged for {key}"
        );
    }
    validate_report(&current)
}

const STRICT_FIELDS: &[&str] = &[
    "strict_policy",
    "strict_files_checked",
    "strict_callables_checked",
    "functions_over_60_lines",
    "functions_over_60_sites",
    "functions_over_25_logical_lines",
    "functions_over_25_sites",
    "functions_over_5_parameters",
    "functions_over_5_parameter_sites",
    "strict_unresolved_sites",
];

fn validate_report(report: &Value) -> Result<()> {
    let structure = report
        .get("structure")
        .context("strict size report missing structure")?;
    ensure!(
        structure.get("strict_policy").and_then(Value::as_str) == Some(POLICY),
        "trusted strict AST size policy measurement missing"
    );
    ensure!(
        structure
            .get("strict_files_checked")
            .and_then(Value::as_u64)
            .is_some_and(|count| count > 0),
        "strict size gate selected zero production files"
    );
    validate_findings(structure)
}

fn validate_findings(structure: &Value) -> Result<()> {
    let lines = validated_sites(
        structure,
        "functions_over_25_logical_lines",
        "functions_over_25_sites",
    )?;
    let parameters = validated_sites(
        structure,
        "functions_over_5_parameters",
        "functions_over_5_parameter_sites",
    )?;
    let legacy = validated_sites(
        structure,
        "functions_over_60_lines",
        "functions_over_60_sites",
    )?;
    let unresolved = sites(structure, "strict_unresolved_sites")?;
    let callables = structure
        .get("strict_callables_checked")
        .and_then(Value::as_u64)
        .context("strict callable count missing or invalid")?;
    ensure!(
        callables > 0 || !unresolved.is_empty(),
        "strict size gate selected zero production callables"
    );
    ensure!(
        legacy.len() <= lines.len(),
        "strict 60-line findings exceed 25-line findings"
    );
    ensure!(
        lines.len() <= usize::try_from(callables)?
            && parameters.len() <= usize::try_from(callables)?,
        "strict advisory counts exceed measured callables"
    );
    let details = [display(legacy)?, display(unresolved)?].join("\n");
    ensure!(
        legacy.is_empty() && unresolved.is_empty(),
        "strict handwritten size certification failed:\n{details}"
    );
    Ok(())
}

fn validated_sites<'a>(structure: &'a Value, metric: &str, key: &str) -> Result<&'a [Value]> {
    let count = structure
        .get(metric)
        .and_then(Value::as_u64)
        .context("strict metric missing or invalid")?;
    let sites = sites(structure, key)?;
    ensure!(
        count == u64::try_from(sites.len())?,
        "strict count/site mismatch for {metric}"
    );
    Ok(sites)
}

fn sites<'a>(structure: &'a Value, key: &str) -> Result<&'a [Value]> {
    let sites = structure
        .get(key)
        .and_then(Value::as_array)
        .context("strict sites missing or invalid")?;
    ensure!(
        sites
            .iter()
            .all(|site| site.as_str().is_some_and(|text| !text.is_empty())),
        "strict site is not a nonempty string"
    );
    Ok(sites)
}

fn display(sites: &[Value]) -> Result<String> {
    sites
        .iter()
        .map(|site| site.as_str().context("strict site is not a string"))
        .collect::<Result<Vec<_>>>()
        .map(|sites| sites.join("\n"))
}
