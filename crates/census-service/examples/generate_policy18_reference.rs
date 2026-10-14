#[path = "../benches/capture_export/corpus.rs"]
mod corpus;
#[path = "policy18_fixture/events.rs"]
mod events;
#[path = "../benches/capture_export/ingest.rs"]
mod ingest;

use anyhow::{ensure, Context, Result};
use census_domain::model::RecruitingCatalogV1;
use census_report::export::ExportDataset;
use census_report::report::Scope;
use census_report::workbook::{self, publication};
use census_store::Store;
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().take(3).collect();
    let [_, destination] = args.as_slice() else {
        anyhow::bail!("usage: generate_policy18_reference NEW_RETAINED_OUTPUT_DIRECTORY");
    };
    let mut policy = Vec::new();
    RecruitingCatalogV1::new().write_policy(&mut policy)?;
    let identity: serde_json::Value = serde_json::from_slice(&policy)?;
    ensure!(
        identity.get("policy_revision").is_none(),
        "generate using the pinned original policy18 implementation"
    );
    generate(Path::new(destination))
}

fn generate(destination: &Path) -> Result<()> {
    std::fs::create_dir(destination).context("a fresh retained-output directory is required")?;
    println!(
        "{}",
        serde_json::json!({
            "following_record": "base_capture_corpus_descriptor_only",
            "counts_scope": "base captured corpus, excluding supplemental policy18 fixtures",
            "execution_scope": "oracle and timed/untimed fields describe the original benchmark; this generator does not execute that oracle or benchmark"
        })
    );
    corpus::report()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let store = Store::open(destination.join("store"))?;
    ingest::run(&store, &runtime)?;
    events::seed(&store)?;
    let dataset = ExportDataset::load(&store)?;
    ensure!(
        dataset.lineage.policy_revision == 18,
        "baseline capture is restricted to policy18"
    );
    let options = workbook::Options {
        grad_year: Some(2027),
        out: Some(destination.join("publication")),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: Vec::new(),
        school_year: ingest::school_year()?,
    };
    let censuses = workbook::Censuses::of(&dataset, &store.out_dir(), &[]);
    let workbook = workbook::build_from(&dataset, &store, &options, &censuses)?;
    let generated = serde_json::json!({
        "record": "generated_policy18_dataset_counts",
        "scope": "base captured corpus plus supplemental policy18 fixtures",
        "athletes": dataset.athletes.len(),
        "events": dataset.events.len(),
        "performances": dataset.performances.len(),
        "publication_verification": "passed"
    });
    drop(dataset);
    drop(censuses);
    publication::verify_published(&workbook)?;
    println!("{generated}");
    println!("{}", workbook.display());
    Ok(())
}
