use anyhow::{Context, Result};
use census_domain::public_identity_history::local_adjudication::RetainedClaimV1;
use census_domain::public_identity_history::{PublicAthleteId, RegistryHead};
use census_store::{LocalAdjudicationCommitOutcomeV1, PublicIdentityWriterV1, Store};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::{approve_prepared, input};

#[derive(Debug, Args)]
pub(in crate::cli) struct LocalAdjudicationArgs {
    #[command(subcommand)]
    command: LocalCommand,
}

#[derive(Debug, Subcommand)]
enum LocalCommand {
    #[command(about = "Read the complete retained claim inventory for a public identity")]
    Inspect {
        #[arg(long, value_name = "PUBLIC_ID", value_parser = PublicAthleteId::parse)]
        canonical: PublicAthleteId,
    },
    #[command(
        about = "Prepare a reviewed split, display its complete record and require exact terminal digest approval before atomic append"
    )]
    Split {
        #[arg(long, value_name = "OPERATION_ID")]
        operation: String,
        #[arg(
            long,
            value_name = "JSON",
            help = "Version-1 split request with exhaustive retained claim dispositions and operator rationale; at most 1 MiB"
        )]
        file: PathBuf,
    },
    #[command(about = "Read a previously committed operation with its exact approved digest")]
    Receipt {
        #[arg(long, value_name = "OPERATION_ID")]
        operation: String,
        #[arg(long, value_name = "SHA256")]
        approval_digest: String,
    },
}

pub(in crate::cli) fn run(cli: &crate::cli::Cli, args: &LocalAdjudicationArgs) -> Result<()> {
    let root = require_root(cli.store.as_deref())?;
    match &args.command {
        LocalCommand::Inspect { canonical } => inspect(root, canonical),
        LocalCommand::Split { operation, file } => split(root, operation, file),
        LocalCommand::Receipt {
            operation,
            approval_digest,
        } => receipt(root, operation, approval_digest),
    }
}

fn require_root(root: Option<&Path>) -> Result<&Path> {
    root.context("local-identity-adjudication requires explicit --store and a stopped store owner")
}

fn open(root: &Path) -> Result<(Store, PublicIdentityWriterV1)> {
    Ok(Store::open_public_identity_qualification_v2(root)
        .context("opening the existing V2 identity qualification store; stop its owner first")?
        .into_parts())
}

fn inspect(root: &Path, canonical: &PublicAthleteId) -> Result<()> {
    let (store, writer) = open(root)?;
    let inventory = writer.inspect_local_adjudication(&store, canonical)?;
    emit(&Inspection {
        schema: "local-identity-adjudication-inventory/v1",
        canonical,
        head: writer.head(&store)?,
        authority_context: inventory.authority_context(),
        claims: inventory.claims(),
    })
}

fn split(root: &Path, operation: &str, file: &Path) -> Result<()> {
    let request = input::read_request(file)?;
    let (store, writer) = open(root)?;
    let head = writer.head(&store)?;
    let prepared = writer.prepare_local_adjudication(
        &store,
        operation,
        request,
        store.snapshot().revision(),
        &head,
    )?;
    let approved = approve_prepared(prepared.record())?;
    let (approved_bytes, approval_digest) = approved.into_approved_record();
    let outcome = writer.apply_local_adjudication(
        &store,
        prepared,
        approved_bytes,
        approval_digest.clone(),
    )?;
    emit_outcome(operation, &approval_digest, &outcome).context(
        "local identity adjudication committed, but output failed; use receipt with the displayed operation ID and approved digest",
    )
}

fn receipt(root: &Path, operation: &str, approval_digest: &str) -> Result<()> {
    let (store, writer) = open(root)?;
    let outcome = writer
        .replay_local_adjudication(&store, operation, approval_digest)?
        .context("no committed local identity adjudication has this operation ID")?;
    emit_outcome(operation, approval_digest, &outcome)
}

#[derive(Serialize)]
struct Inspection<'a> {
    schema: &'static str,
    canonical: &'a PublicAthleteId,
    head: RegistryHead,
    authority_context: &'a str,
    claims: &'a [RetainedClaimV1],
}

#[derive(Serialize)]
struct Outcome<'a> {
    schema: &'static str,
    operation: &'a str,
    approval_digest: &'a str,
    head: &'a RegistryHead,
    introduced_public_ids: &'a [PublicAthleteId],
    replayed: bool,
}

fn emit_outcome(
    operation: &str,
    approval_digest: &str,
    outcome: &LocalAdjudicationCommitOutcomeV1,
) -> Result<()> {
    emit(&Outcome {
        schema: "local-identity-adjudication-outcome/v1",
        operation,
        approval_digest,
        head: outcome.head(),
        introduced_public_ids: outcome.introduced_public_ids(),
        replayed: outcome.replayed(),
    })
}

fn emit(value: &impl Serialize) -> Result<()> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, value)
        .context("encoding local identity adjudication output")?;
    output
        .write_all(b"\n")
        .context("writing local identity adjudication output")?;
    output
        .flush()
        .context("flushing local identity adjudication output")
}

#[cfg(test)]
mod tests;
