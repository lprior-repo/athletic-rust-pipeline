mod common;

#[path = "parity_pipeline_mod/assertions.rs"]
mod assertions;
#[path = "parity_pipeline_mod/constants.rs"]
mod constants;
#[path = "parity_pipeline_mod/fixture_builders.rs"]
mod fixture_builders;
#[path = "parity_pipeline_mod/fixtures.rs"]
mod fixtures;
#[path = "parity_pipeline_mod/ohsaa_wiaa_builders.rs"]
mod ohsaa_wiaa_builders;
#[path = "parity_pipeline_mod/pipeline.rs"]
mod pipeline;
#[path = "parity_pipeline_mod/wiaa_results.rs"]
mod wiaa_results;

use anyhow::{ensure, Context, Result};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rebuilding_the_fixture_store_reproduces_semantics_not_publication_identity() -> Result<()>
{
    let scratch = tempfile::tempdir().context("temp dir for the runs")?;
    let root = scratch.path().join("store");
    let first = pipeline::run_pipeline(&root).await?;
    std::fs::remove_dir_all(&root)
        .with_context(|| format!("clearing {} between runs", root.display()))?;
    let second = pipeline::run_pipeline(&root).await?;

    ensure!(
        first.counts == second.counts,
        "the consolidate counts changed when the store was rebuilt"
    );
    ensure!(
        pipeline::report_projection(&first.results_report)
            == pipeline::report_projection(&second.results_report),
        "the result-file adapter's report changed when the store was rebuilt"
    );

    ensure!(
        first.publication.workbook.rows == second.publication.workbook.rows
            && first.publication.workbook.mapped_athletes
                == second.publication.workbook.mapped_athletes,
        "rebuilding the same source corpus changed the verified published record population"
    );
    ensure!(
        first.publication.generation_digest != second.publication.generation_digest,
        "a rebuilt store must not inherit the earlier store's publication identity"
    );
    Ok(())
}
