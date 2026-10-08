use super::*;
use anyhow::Context;
use std::path::PathBuf;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn function(lines: usize) -> String {
    format!(
        "fn boundary() {{\n{}}}\n",
        "consume();\n".repeat(lines.saturating_sub(2))
    )
}

fn report(found: Findings) -> Value {
    let mut structure = Map::new();
    found.insert(&mut structure);
    serde_json::json!({ "structure": structure, "crates": {} })
}

mod boundaries;
#[path = "tests/modules.rs"]
mod classification;
mod exclusions;
mod gate;
#[path = "tests/provenance.rs"]
mod provenance;
