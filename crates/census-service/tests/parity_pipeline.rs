mod common;

#[path = "parity_pipeline_mod/constants.rs"]
mod constants;
#[path = "parity_pipeline_mod/pipeline.rs"]
mod pipeline;
#[path = "parity_pipeline_mod/fixtures.rs"]
mod fixtures;
#[path = "parity_pipeline_mod/wiaa_results.rs"]
mod wiaa_results;
#[path = "parity_pipeline_mod/fixture_builders.rs"]
mod fixture_builders;
#[path = "parity_pipeline_mod/ohsaa_wiaa_builders.rs"]
mod ohsaa_wiaa_builders;
#[path = "parity_pipeline_mod/assertions.rs"]
mod assertions;
#[path = "parity_pipeline_mod/utils.rs"]
mod utils;
#[path = "parity_pipeline_mod/workbook.rs"]
mod workbook;

use anyhow::{ensure, Context, Result};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pipeline_publishes_the_same_bytes_from_a_rebuilt_store() -> Result<()> {
    let scratch = tempfile::tempdir().context("temp dir for the runs")?;
    let root = scratch.path().join("store");
    let first = pipeline::run_pipeline(&root).await?;
    std::fs::remove_dir_all(&root)
        .with_context(|| format!("clearing {} between runs", root.display()))?;
    let second = pipeline::run_pipeline(&root).await?;


    ensure!(
        first.report_text == second.report_text,
        "the census JSON changed when the store was rebuilt"
    );
    ensure!(
        first.census_by_state_core == second.census_by_state_core
            && first.census_by_state_all_sources == second.census_by_state_all_sources,
        "a per-state CSV changed when the store was rebuilt"
    );
    ensure!(
        first.bests_jsonl == second.bests_jsonl && first.bests_csv == second.bests_csv,
        "a best-mark sidecar changed when the store was rebuilt"
    );
    ensure!(
        first.counts == second.counts,
        "the consolidate counts changed when the store was rebuilt"
    );
    ensure!(
        pipeline::report_projection(&first.results_report)
            == pipeline::report_projection(&second.results_report),
        "the result-file adapter's report changed when the store was rebuilt"
    );

    anyhow::ensure!(
        first.workbook.parts == second.workbook.parts,
        "an xlsx part changed when the store was rebuilt — left={:?} right={:?}",
        &first.workbook.parts,
        &second.workbook.parts
    );
    ensure!(
        utils::part_bytes(&first.workbook.bytes, constants::CORE_PART)?
            == utils::part_bytes(&second.workbook.bytes, constants::CORE_PART)?,
        "the two workbooks differ outside {}, the one part rust_xlsxwriter stamps with the file's \
         creation time",
        constants::CORE_PART
    );
    ensure!(
        first.workbook.shape == second.workbook.shape,
        "the workbook's sheets or cells changed when the store was rebuilt"
    );
    Ok(())
}
