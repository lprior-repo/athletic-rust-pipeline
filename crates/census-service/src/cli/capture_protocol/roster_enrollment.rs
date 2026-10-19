use anyhow::{ensure, Context, Result};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{RosterEnrollmentReviewV1, RosterReviewActorV1, Store};
use clap::Args;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Args)]
pub(super) struct RosterEnrollmentArgs {
    #[arg(long)]
    operation: String,
    #[arg(long)]
    reviewed_at: String,
    #[arg(long)]
    rationale: String,
    #[arg(long)]
    approve_sha256: Option<String>,
}

pub(super) fn run_to<W: Write>(
    root: &Path,
    args: &RosterEnrollmentArgs,
    output: &mut W,
) -> Result<()> {
    let review = RosterEnrollmentReviewV1 {
        operation: RosterAuthorityTextV1::parse(&args.operation)?,
        actor: RosterReviewActorV1::LocalOperator,
        reviewed_at: chrono::DateTime::parse_from_rfc3339(&args.reviewed_at)
            .context("parse reviewed-at RFC3339 timestamp")?
            .to_utc(),
        rationale: RosterAuthorityTextV1::parse(&args.rationale)?,
    };
    let (store, _writer) = Store::open_current_qualification_v3(root)?.into_parts();
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(inputs, review)?;
    let Some(given_digest) = args.approve_sha256.as_deref() else {
        output.write_all(prepared.approval_bytes())?;
        return Ok(());
    };
    ensure!(
        given_digest == prepared.approval_digest(),
        "roster enrollment approval differs from the inspected state and review"
    );
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let applied = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    serde_json::to_writer_pretty(&mut *output, applied.receipt())?;
    writeln!(output)?;
    Ok(())
}
