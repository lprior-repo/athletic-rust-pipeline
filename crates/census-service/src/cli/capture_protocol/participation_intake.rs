use anyhow::{Context, Result};
use census_crawl::participation_evidence::{recorded, IndependentParticipationPacketV1};
use census_store::Store;
use clap::Args;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
pub(super) struct ParticipationIntakeArgs {
    #[arg(long, value_name = "PATH")]
    packet: PathBuf,
    #[arg(long, value_name = "OPERATION")]
    operation: String,
}

fn read_packet(path: &Path) -> Result<IndependentParticipationPacketV1> {
    let bytes =
        census_crawl::convert::read_file_capped(path, recorded::MAX_PROVIDER_SCHOOL_PACKET_BYTES)
            .context("reading the bounded independent participation packet")?;
    serde_json::from_slice(&bytes).context("decoding the independent participation packet")
}

pub(super) fn run_to<W: Write>(
    root: &Path,
    args: &ParticipationIntakeArgs,
    output: &mut W,
) -> Result<()> {
    let operation =
        census_domain::roster_affiliation_authority::RosterAuthorityTextV1::parse(&args.operation)
            .context("parsing the stable intake operation")?;
    let packet = read_packet(&args.packet)?;
    let reference = packet
        .reference()
        .context("checking the packet reference")?;
    let capture_references = packet
        .capture_references()
        .context("checking the packet capture references")?;
    let (store, _writer) = Store::open_current_qualification_v3(root)
        .context("opening the stopped current-qualification store")?
        .into_parts();
    let recording = census_crawl::Recording::new();
    recorded::record_independent_participation_packet(&store, &recording, &packet)
        .context("staging the unreviewed independent participation packet")?;
    let staged_rows = recording.rows();
    let prepared = census_crawl::recording::prepare_recorded(recording.drain(), operation.as_str())
        .context("preparing the unreviewed intake recording")?;
    let applied = prepared
        .apply(&store)
        .context("applying the intake recording")?;
    serde_json::to_writer_pretty(
        &mut *output,
        &serde_json::json!({
            "schema": "participation-intake-v1",
            "status": "intaken_unreviewed",
            "packet_reference": reference.as_str(),
            "capture_references": capture_references,
            "requested_operation": operation.as_str(),
            "operation": prepared.operation(),
            "staged_rows": staged_rows,
            "appended": applied.appended(),
            "repeated": applied.repeated(),
            "receipt_operation": applied.receipt().operation,
            "receipt_digest": applied.receipt().digest,
            "qualification_scope": "Unreviewed independent participation packet only; no identity, affiliation, calendar, eligibility or current-school acceptance"
        }),
    )?;
    writeln!(output)?;
    Ok(())
}

#[cfg(test)]
mod tests;
