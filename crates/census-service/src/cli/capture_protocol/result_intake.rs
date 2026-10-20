use anyhow::{Context, Result};
use census_crawl::milesplit::SourceRowLocator;
use census_crawl::participation_evidence::recorded::{
    record_retained_html_result_capture, MAX_PROVIDER_SCHOOL_PACKET_BYTES,
};
use census_crawl::participation_evidence::result_ownership::read_retained_html_result_rows;
use census_store::Store;
use clap::Args;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
pub(super) struct ResultIntakeArgs {
    #[arg(long, value_name = "PATH")]
    capture: PathBuf,
    #[arg(long, value_name = "PATH")]
    rows: PathBuf,
    #[arg(long, value_name = "OPERATION")]
    operation: String,
}

pub(super) fn run_to<W: Write>(root: &Path, args: &ResultIntakeArgs, output: &mut W) -> Result<()> {
    let operation =
        census_domain::roster_affiliation_authority::RosterAuthorityTextV1::parse(&args.operation)
            .context("parsing the stable intake operation")?;

    let capture_bytes =
        census_crawl::convert::read_file_capped(&args.capture, MAX_PROVIDER_SCHOOL_PACKET_BYTES)
            .context("reading the bounded original HTML result capture")?;
    let rows_bytes =
        census_crawl::convert::read_file_capped(&args.rows, MAX_PROVIDER_SCHOOL_PACKET_BYTES)
            .context("reading the bounded original row locator list")?;

    let capture: census_crawl::participation_evidence::RetainedHtmlCaptureV1 =
        serde_json::from_slice(&capture_bytes)
            .context("decoding the canonical original HTML result capture")?;
    let requested: Vec<SourceRowLocator> = serde_json::from_slice(&rows_bytes)
        .context("decoding the canonical original row locator list")?;

    let document = read_retained_html_result_rows(&capture, &requested)
        .context("validating retained original result row ownership")?;

    let (store, _writer) = Store::open_current_qualification_v3(root)
        .context("opening the stopped current-qualification store")?
        .into_parts();

    let recording = census_crawl::Recording::new();
    record_retained_html_result_capture(&store, &recording, &document)
        .context("staging the unreviewed original result document")?;

    let retained_rows = document.selected_count();
    let staged_rows = recording.rows();
    let prepared = census_crawl::recording::prepare_recorded(recording.drain(), operation.as_str())
        .context("preparing the unreviewed intake recording")?;
    let applied = prepared
        .apply(&store)
        .context("applying the intake recording")?;

    serde_json::to_writer_pretty(
        &mut *output,
        &serde_json::json!({
            "schema": "result-intake",
            "status": "intaken_unreviewed",
            "context_reference": document.context_reference(),
            "capture_url": capture.requested_url(),
            "capture_sha256": capture.body_sha256(),
            "requested_rows": requested.len(),
            "retained_rows": retained_rows,
            "meet_id": document.meet_id(),
            "result_document_id": document.result_document_id(),
            "published_start": document.published_start(),
            "published_end": document.published_end(),
            "requested_operation": operation.as_str(),
            "operation": prepared.operation(),
            "staged_rows": staged_rows,
            "appended": applied.appended(),
            "repeated": applied.repeated(),
            "receipt_operation": applied.receipt().operation,
            "receipt_digest": applied.receipt().digest,
            "qualification_scope": "Unreviewed original retained result document only; no identity, eligibility or current-school acceptance"
        }),
    )?;
    writeln!(output)?;
    Ok(())
}

#[cfg(test)]
mod provenance_tests;
#[cfg(test)]
mod tests;
