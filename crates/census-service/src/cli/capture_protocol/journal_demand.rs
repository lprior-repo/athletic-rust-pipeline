use anyhow::{Context, Result};
use census_crawl::milesplit::RetainedJournalBundle;
use census_store::{Store, StoreError};
use std::io::{self, Write};
use std::path::Path;

pub(super) fn run_to<W: Write>(root: &Path, output: &mut W) -> Result<()> {
    run_with_progress(root, output, |stage| {
        write_progress(&mut io::stderr().lock(), stage)
    })
}

fn run_with_progress<W: Write>(
    root: &Path,
    output: &mut W,
    mut progress: impl FnMut(&'static str) -> io::Result<()>,
) -> Result<()> {
    progress("store_open_started").context("reporting journal demand progress")?;
    let (store, _writer) = Store::open_current_qualification_v3(root)
        .context("opening the stopped current-qualification store")?
        .into_parts();
    progress("store_open_completed").context("reporting journal demand progress")?;
    let view = store.snapshot();
    progress("snapshot_held").context("reporting journal demand progress")?;
    let value = {
        let demand = RetainedJournalBundle::measure_present_with_progress(&view, |stage| {
            progress(stage).map_err(|source| StoreError::Io {
                path: "journal-demand progress".into(),
                source,
            })
        })
        .context("measuring present retained source-journal values")?;
        serde_json::json!({
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
        })
    };
    let bytes = serde_json::to_vec(&value).context("encoding journal demand scalars")?;
    anyhow::ensure!(
        bytes.len() <= 16 * 1024,
        "journal demand output exceeds 16KiB"
    );
    output
        .write_all(&bytes)
        .context("writing complete journal demand")?;
    writeln!(output).context("finishing complete journal demand")?;
    output.flush().context("flushing complete journal demand")?;
    progress("output_completed").context("reporting journal demand progress")
}

fn write_progress<W: Write>(output: &mut W, stage: &'static str) -> io::Result<()> {
    if !matches!(
        stage,
        "store_open_started"
            | "store_open_completed"
            | "snapshot_held"
            | "capture_phase_started"
            | "capture_phase_completed"
            | "meet_phase_started"
            | "meet_phase_completed"
            | "counter_completed"
            | "output_completed"
    ) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unknown journal demand stage",
        ));
    }
    let mut bytes = serde_json::to_vec(&serde_json::json!({
        "schema": "journal-demand-progress-v1",
        "kind": "journal_demand_progress",
        "stage": stage,
    }))
    .map_err(io::Error::other)?;
    bytes.push(b'\n');
    if bytes.len() > 512 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "journal demand progress exceeds 512 bytes",
        ));
    }
    output.write_all(&bytes)?;
    output.flush()
}

#[cfg(test)]
mod tests;
