use anyhow::{Context, Result};
use census_crawl::participation_evidence::{provider_school::ProviderSchoolPacketV1, recorded};
use census_store::Store;
use clap::Args;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
pub(super) struct RosterIntakeArgs {
    #[arg(long, value_name = "PATH")]
    packet: PathBuf,
    #[arg(long, value_name = "OPERATION")]
    operation: String,
}

pub(super) fn run_to<W: Write>(root: &Path, args: &RosterIntakeArgs, output: &mut W) -> Result<()> {
    let operation =
        census_domain::roster_affiliation_authority::RosterAuthorityTextV1::parse(&args.operation)
            .context("parsing the stable intake operation")?;
    let bytes = census_crawl::convert::read_file_capped(
        &args.packet,
        recorded::MAX_PROVIDER_SCHOOL_PACKET_BYTES,
    )
    .context("reading the bounded original provider-school packet")?;
    let packet = ProviderSchoolPacketV1::decode(&bytes)
        .context("decoding the canonical original provider-school packet")?;
    let (store, _writer) = Store::open_current_qualification_v3(root)
        .context("opening the stopped current-qualification store")?
        .into_parts();
    let recording = census_crawl::Recording::new();
    let school_id = recorded::record_provider_school_intake(&store, &recording, &packet)
        .context("staging the unreviewed school facts")?;
    let checked = recorded::record_provider_roster_rows_v3(&store, &recording, &packet)
        .context("staging the unreviewed roster rows")?;
    let disposition = match checked.roster_verdict() {
        census_crawl::milesplit::RosterVerdict::Complete { .. } => "complete",
        census_crawl::milesplit::RosterVerdict::Partial { .. } => "partial",
        census_crawl::milesplit::RosterVerdict::Quarantined { .. } => "quarantined",
    };
    let staged_rows = recording.rows();
    let prepared = census_crawl::recording::prepare_recorded(recording.drain(), operation.as_str())
        .context("preparing the unreviewed intake recording")?;
    let applied = prepared
        .apply(&store)
        .context("applying the intake recording")?;
    serde_json::to_writer_pretty(
        &mut *output,
        &serde_json::json!({
            "schema": "roster-intake-v1",
            "status": "intaken_unreviewed",
            "packet_reference": checked.reference().as_str(),
            "requested_operation": operation.as_str(),
            "operation": prepared.operation(),
            "school_id": school_id,
            "roster_disposition": disposition,
            "admitted_roster_rows": checked.roster_verdict().roster().map_or(0, |roster| roster.athletes.len()),
            "rejected_roster_rows": checked.roster_verdict().rejections().len(),
            "roster_unfinished": checked.roster_verdict().unfinished().is_some(),
            "staged_rows": staged_rows,
            "appended": applied.appended(),
            "repeated": applied.repeated(),
            "receipt_operation": applied.receipt().operation,
            "receipt_digest": applied.receipt().digest,
            "qualification_scope": "Unreviewed facts only; no identity, eligibility or current-school acceptance"
        }),
    )?;
    writeln!(output)?;
    Ok(())
}
