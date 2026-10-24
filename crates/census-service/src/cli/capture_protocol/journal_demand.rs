use anyhow::{Context, Result};
use census_crawl::milesplit::RetainedJournalBundle;
use census_store::Store;
use std::io::Write;
use std::path::Path;

pub(super) fn run_to<W: Write>(root: &Path, output: &mut W) -> Result<()> {
    let (store, _writer) = Store::open_current_qualification_v3(root)
        .context("opening the stopped current-qualification store")?
        .into_parts();
    let view = store.snapshot();
    let demand = RetainedJournalBundle::measure_present(&view)
        .context("measuring present retained source-journal values")?;
    let value = serde_json::json!({
        "schema": "retained-journal-present-demand-v1",
        "demand": demand,
        "unmeasured": [
            "absent_and_legacy_probes",
            "original_job_binding",
            "actual_minimum_read_grants",
            "outer_replay_v4",
            "attachment_closure",
            "descriptor_and_cold_decoder_fit",
            "rss_and_workbook_fit"
        ]
    });
    let bytes = serde_json::to_vec(&value).context("encoding journal demand scalars")?;
    anyhow::ensure!(
        bytes.len() <= 16 * 1024,
        "journal demand output exceeds 16KiB"
    );
    output
        .write_all(&bytes)
        .context("writing complete journal demand")?;
    writeln!(output).context("finishing complete journal demand")
}

#[cfg(test)]
mod tests;
