use census_crawl::milesplit::{
    read_retained_acquisition, CaptureReadBudget, OwnedAcquisitionRef, VerifiedOwnedAcquisition,
};
use census_store::Store;
use clap::Args;
use serde::Serialize;
use std::io::Write;
use std::path::Path;

mod error;
#[cfg(test)]
mod tests;

use error::{CaptureCliError, CaptureCliResult};

#[derive(Debug, Args)]
pub(super) struct RetainedCaptureArgs {
    #[arg(long, value_name = "REFERENCE")]
    acquisition: String,
    #[arg(long, default_value_t = census_crawl::milesplit::MAX_OWNED_BODY_BYTES)]
    max_body_bytes: usize,
}

pub(super) fn run(cli: &super::Cli, args: &RetainedCaptureArgs) -> anyhow::Result<()> {
    run_to(cli.store.as_deref(), args, &mut std::io::stdout().lock()).map_err(Into::into)
}

fn run_to<W: Write>(
    root: Option<&Path>,
    args: &RetainedCaptureArgs,
    writer: &mut W,
) -> CaptureCliResult<()> {
    match execute(root, args, writer) {
        Ok(()) => Ok(()),
        Err(error) if error.output_failed() => Err(error),
        Err(error) => {
            emit(
                writer,
                &Failure {
                    schema_version: "retained_capture_verification_v1",
                    status: "failed",
                    code: error.code(),
                    message: &error.to_string(),
                },
            )?;
            Err(error)
        }
    }
}

fn execute<W: Write>(
    root: Option<&Path>,
    args: &RetainedCaptureArgs,
    writer: &mut W,
) -> CaptureCliResult<()> {
    let root = root.ok_or(CaptureCliError::StoreRequired)?;
    let reference = OwnedAcquisitionRef::parse(&args.acquisition)?;
    let budget = CaptureReadBudget::new(args.max_body_bytes)?;
    let store = Store::open_existing(root)?;
    let snapshot = store.snapshot();
    let verified = read_retained_acquisition(&snapshot, &reference, budget)?;
    emit(writer, &Reply::new(&verified, snapshot.sequence()))
}

#[derive(Serialize)]
struct Reply<'a> {
    schema_version: &'static str,
    status: &'static str,
    acquisition: &'a str,
    meet_id: u64,
    snapshot_sequence: u64,
    capture: &'a census_crawl::net::FetchOutcome,
    verified_body_bytes: usize,
    qualification_scope: &'static str,
}

impl<'a> Reply<'a> {
    fn new(verified: &'a VerifiedOwnedAcquisition, snapshot_sequence: u64) -> Self {
        Self {
            schema_version: "retained_capture_verification_v1",
            status: "verified",
            acquisition: verified.reference().manifest_key(),
            meet_id: verified.reference().meet_id(),
            snapshot_sequence,
            capture: verified.capture(),
            verified_body_bytes: verified.body().len(),
            qualification_scope: "capture bytes and occurrence only; interpretation, identity, cohort, affiliation and coverage remain unqualified",
        }
    }
}

#[derive(Serialize)]
struct Failure<'a> {
    schema_version: &'static str,
    status: &'static str,
    code: &'static str,
    message: &'a str,
}

fn emit<W: Write, T: Serialize>(writer: &mut W, value: &T) -> CaptureCliResult<()> {
    serde_json::to_writer(&mut *writer, value).map_err(CaptureCliError::OutputJson)?;
    writer.write_all(b"\n").map_err(CaptureCliError::OutputIo)?;
    writer.flush().map_err(CaptureCliError::OutputIo)
}
