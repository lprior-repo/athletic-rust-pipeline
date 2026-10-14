use anyhow::{ensure, Context, Result};
use serde_json::{Map, Value};
use std::path::Path;

#[path = "csv_tests.rs"]
mod tests;

const FAMILIES: [&str; 8] = [
    "athletes.csv",
    "best_marks.csv",
    "schools.csv",
    "coach_contacts.csv",
    "comparison_classes.csv",
    "provenance.csv",
    "coverage.csv",
    "identity_aliases.csv",
];

pub(super) fn require_families(inventory: &Map<String, Value>) -> Result<()> {
    for name in FAMILIES {
        ensure!(
            inventory.contains_key(name),
            "missing normalized family {name}"
        );
    }
    Ok(())
}

pub(super) fn read(path: &Path, name: &str, input_generation: &str) -> Result<Vec<Vec<String>>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)?;
    let mut rows = reader
        .records()
        .map(|row| row.map(|row| row.iter().map(str::to_string).collect()))
        .collect::<std::result::Result<Vec<Vec<String>>, csv::Error>>()?;
    if FAMILIES.contains(&name) {
        normalize_binding(&mut rows, name, input_generation)?;
    }
    Ok(rows)
}

fn normalize_binding(rows: &mut [Vec<String>], name: &str, input_generation: &str) -> Result<()> {
    ensure!(
        !input_generation.is_empty(),
        "empty publication input generation"
    );
    let header = rows
        .first()
        .with_context(|| format!("missing {name} header"))?;
    let bindings = header
        .iter()
        .filter(|field| field.as_str() == "input_generation")
        .count();
    ensure!(
        bindings == 1,
        "{name}: expected one input_generation column"
    );
    let index = header
        .iter()
        .position(|field| field == "input_generation")
        .context("input generation column")?;
    for (ordinal, row) in rows.iter_mut().enumerate().skip(1) {
        let binding = row
            .get_mut(index)
            .with_context(|| format!("{name} row {ordinal}: missing input generation"))?;
        ensure!(
            binding.as_str() == input_generation,
            "{name} row {ordinal}: input generation {binding:?} differs from manifest {input_generation:?}"
        );
        binding.clear();
    }
    Ok(())
}
