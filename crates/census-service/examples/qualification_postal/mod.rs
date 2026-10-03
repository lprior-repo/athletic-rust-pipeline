mod acquisition;
mod artifacts;
mod cli;
mod postal;
mod publication;

use anyhow::{ensure, Context};
use census_report::export::ExportDataset;
use census_store::Store;
use serde_json::json;

use artifacts::{write_json, Result};

const MODEL: &str = "openai-codex/gpt-6.1-sol";
const CAPTURED: &str = "2026-09-27T00:00:00Z";
const NAME: &str = "A.C. Reynolds High School";
const ORG: &str = "6285bff87f770402d0000006";

pub(super) fn execute() -> Result<()> {
    let (root, binary, observed) = cli::arguments()?;
    std::fs::create_dir(&root)
        .context("qualification requires a new exclusive directory; reuse refused")?;
    let root = std::fs::canonicalize(root)?;
    let sources = artifacts::save_sources(&root)?;
    let store = Store::open(root.join("store"))?;
    let metadata = acquisition::run(&root, &store, &observed)?;
    store.flush()?;
    let dataset = ExportDataset::load(&store)?;
    dataset.ensure_current(&store)?;
    postal::verify_stored(&dataset, CAPTURED)?;
    dataset.save_frozen(&root.join("frozen-input.json"))?;
    let (mut evidence, xlsx) =
        publication::publish_and_readback(&root, &store, &dataset, &observed, &metadata)?;
    store.flush()?;
    drop(store);
    let invocation = cli::run_csv(&root, &binary)?;
    let school = dataset.schools.values().next().context("school absent")?;
    let csv = publication::read_csv(&root.join("data/canonical-schools.csv"), school, CAPTURED)?;
    ensure!(
        csv == xlsx,
        "CSV and independently read Schools XLSX postal cells differ"
    );
    write_json(
        &root.join("csv-readback.json"),
        &serde_json::to_value(&csv)?,
    )?;
    let object = evidence
        .as_object_mut()
        .context("qualification evidence is not an object")?;
    object.insert("csv_cli".into(), invocation);
    object.insert("source_snapshot".into(), sources);
    object.insert("status".into(), json!("passed"));
    write_json(&root.join("qualification.json"), &evidence)?;
    println!("POSTAL QUALIFICATION PASSED: {} (one captured school, two claims, 16 coaches, zero athletes; not a national census)", root.display());
    Ok(())
}
