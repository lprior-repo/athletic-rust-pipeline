use anyhow::{ensure, Context, Result};
use census_domain::roster_affiliation_authority::{RosterAuthorityDigestV1, RosterAuthorityTextV1};
use census_store::Store;
use clap::Args;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Args)]
pub(super) struct RosterReplayArgs {
    #[arg(
        long,
        value_name = "OPERATION",
        help = "Exact operation label of the previously applied roster source review; bounded review text"
    )]
    operation: String,
    #[arg(
        long,
        value_name = "SHA256",
        help = "SHA256 of the original opaque approval bytes: 64 lowercase hex characters"
    )]
    approval_sha256: String,
}

pub(super) fn run_to<W: Write>(root: &Path, args: &RosterReplayArgs, output: &mut W) -> Result<()> {
    let (operation, approval_digest) = parse(args)?;
    let (store, _writer) = Store::open_current_qualification_v3(root)
        .context("opening the stopped current-qualification store")?
        .into_parts();
    let applied = store
        .replay_roster_source_review_v1(operation.as_str(), approval_digest.as_str())
        .context("replaying the original roster source review receipt")?;
    let receipt = applied.receipt();
    let reply = serde_json::json!({
        "schema": "roster-replay",
        "status": "replayed",
        "operation": operation,
        "approval_digest": approval_digest,
        "receipt_operation": receipt.operation,
        "receipt_digest": receipt.digest,
        "receipt_at": receipt.at,
        "repeated": applied.repeated(),
        "appended": applied.appended(),
        "qualification_scope": "original receipt lookup for a previously committed roster source review; validates the retained record, closure, chain and original receipt against the supplied approval digest, and does not re-inspect, re-prepare, re-approve, recapture, create a new operation, or confer current membership, eligibility or national acceptance",
    });
    serde_json::to_writer_pretty(&mut *output, &reply)?;
    writeln!(output)?;
    Ok(())
}

fn parse(args: &RosterReplayArgs) -> Result<(RosterAuthorityTextV1, RosterAuthorityDigestV1)> {
    let operation = RosterAuthorityTextV1::parse(&args.operation)
        .context("parsing the original roster review operation")?;
    ensure!(
        operation.as_str().len() <= 256,
        "review operation exceeds 256 bytes"
    );
    let approval_digest = RosterAuthorityDigestV1::parse(&args.approval_sha256)
        .context("parsing the original approval SHA256")?;
    Ok((operation, approval_digest))
}
