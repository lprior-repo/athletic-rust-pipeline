use anyhow::{Context, Result};
use census_domain::public_identity_history::local_adjudication::RetainedClaimV1;
use census_domain::public_identity_history::{PublicAthleteId, RegistryHead};
use census_store::{LocalAdjudicationCommitOutcomeV1, PublicIdentityWriterV1, Store};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::{approve_prepared, input};
mod demand;
mod source;

#[derive(Debug, Args)]
pub(in crate::cli) struct LocalAdjudicationArgs {
    #[command(subcommand)]
    command: LocalCommand,
}

#[derive(Debug, Subcommand)]
enum LocalCommand {
    #[command(about = "Inspect the retained format of an existing stopped V3 qualification store")]
    InspectStoreFormat,
    #[command(
        about = "Measure complete frozen identity JSON demand without creating a frozen bundle or publication"
    )]
    MeasureFrozenPublicIdentity,
    #[command(
        about = "Explicitly enroll bounded graph admission and all six input revisions for a fresh V3 qualification store"
    )]
    EnrollGraphAdmission,
    #[command(
        about = "Stage the bounded retained graph and read its complete claim inventory for an explicit logical operation"
    )]
    Inspect {
        #[arg(long, value_name = "OPERATION_ID")]
        operation: String,
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
    #[command(
        about = "Stage the bounded retained graph and preview the exact detached membership set for an explicit logical operation"
    )]
    PreviewMembership {
        #[arg(long, value_name = "OPERATION_ID")]
        operation: String,
        #[arg(long, value_parser = PublicAthleteId::parse)]
        canonical: PublicAthleteId,
        #[arg(long)]
        sequence: u64,
        #[arg(long)]
        digest: String,
        #[arg(long)]
        membership_id: String,
    },
    #[command(
        about = "Withhold one exact source-candidate attachment after complete local terminal review"
    )]
    WithholdMembership {
        #[arg(long, value_name = "OPERATION_ID")]
        operation: String,
        #[arg(long, value_name = "JSON")]
        file: PathBuf,
    },
    #[command(about = "Read an approved source-membership withholding receipt")]
    MembershipReceipt {
        #[arg(long, value_name = "OPERATION_ID")]
        operation: String,
        #[arg(long, value_name = "SHA256")]
        approval_digest: String,
    },
    #[command(about = "Read a previously committed operation with its exact approved digest")]
    Receipt {
        #[arg(long, value_name = "OPERATION_ID")]
        operation: String,
        #[arg(long, value_name = "SHA256")]
        approval_digest: String,
    },
    #[command(about = "Establish source-bound identity from candidate with exact source binding")]
    EstablishSource {
        #[arg(long, value_name = "OPERATION_ID")]
        operation: String,
        #[arg(
            long = "subject",
            value_name = "SUBJECT",
            help = "Retained source athlete subject identifier"
        )]
        candidate: String,
    },
    #[command(about = "Establish source-bound identities for one retained graduation cohort")]
    EstablishSourceCohort {
        #[arg(long, value_name = "OPERATION_ID")]
        operation: String,
        #[arg(long, value_parser = clap::value_parser!(i16).range(2020..=2040))]
        graduation_year: i16,
    },
    #[command(
        about = "Measure an existing complete cohort graph without staging or accepting identities"
    )]
    MeasureRetainedSourceCohort {
        #[arg(long, value_name = "ORIGINAL_OPERATION_ID")]
        operation: String,
        #[arg(long, value_parser = clap::value_parser!(i16).range(2020..=2040))]
        graduation_year: i16,
    },
}

pub(in crate::cli) fn run(cli: &crate::cli::Cli, args: &LocalAdjudicationArgs) -> Result<()> {
    let root = require_root(cli.store.as_deref())?;
    match &args.command {
        LocalCommand::InspectStoreFormat => {
            let (store, _writer) = open(root)?;
            emit(&store.store_format()?)
        }
        LocalCommand::MeasureFrozenPublicIdentity => demand::run(root),
        LocalCommand::EnrollGraphAdmission => {
            let (store, writer) = open(root)?;
            enroll_graph_admission(&store, &writer)?;
            emit(&serde_json::json!({
                "schema": "local-identity-graph-admission/v1",
                "qualification_lifecycle": 7,
                "manual_context_admission_version": 2,
                "input_revisions_version": 2,
                "enrolled": true,
            }))
        }
        LocalCommand::Inspect {
            operation,
            canonical,
        } => inspect(root, operation, canonical),
        LocalCommand::Split { operation, file } => split(root, operation, file),
        LocalCommand::PreviewMembership {
            operation,
            canonical,
            sequence,
            digest,
            membership_id,
        } => {
            super::membership::preview(root, operation, canonical, *sequence, digest, membership_id)
        }
        LocalCommand::WithholdMembership { operation, file } => {
            super::membership::withhold(root, operation, file)
        }
        LocalCommand::MembershipReceipt {
            operation,
            approval_digest,
        } => super::membership::receipt(root, operation, approval_digest),
        LocalCommand::Receipt {
            operation,
            approval_digest,
        } => receipt(root, operation, approval_digest),
        LocalCommand::EstablishSource {
            operation,
            candidate,
        } => source::establish(root, operation, candidate),
        LocalCommand::EstablishSourceCohort {
            operation,
            graduation_year,
        } => source::cohort(root, operation, *graduation_year),
        LocalCommand::MeasureRetainedSourceCohort {
            operation,
            graduation_year,
        } => source::measure(root, operation, *graduation_year),
    }
}

fn enroll_graph_admission(store: &Store, writer: &PublicIdentityWriterV1) -> Result<()> {
    writer.enroll_graph_admission_v1(store, store.snapshot().revision())
        .context("graph enrollment did not complete; resolve the reported error and retry enroll-graph-admission on the same store")?;
    Ok(())
}

fn require_root(root: Option<&Path>) -> Result<&Path> {
    root.context("local-identity-adjudication requires explicit --store and a stopped store owner")
}

pub(super) fn open(root: &Path) -> Result<(Store, PublicIdentityWriterV1)> {
    Ok(Store::open_public_identity_qualification_v3(root)
        .context("opening the existing V3 identity qualification store; stop its owner first")?
        .into_parts())
}

fn inspect(root: &Path, operation: &str, canonical: &PublicAthleteId) -> Result<()> {
    let (store, writer) = open(root)?;
    let inventory = writer.inspect_local_adjudication(&store, operation, canonical)?;
    emit(&Inspection {
        schema: "local-identity-adjudication-inventory/v3",
        operation,
        canonical,
        head: writer.head(&store)?,
        graph: inventory.graph(),
        affected_record_keys: inventory.affected_record_keys(),
        claims: inventory.claims(),
    })
}

fn split(root: &Path, operation: &str, file: &Path) -> Result<()> {
    let request = input::read_request(file)?;
    let (store, writer) = open(root)?;
    let head = writer.head(&store)?;
    let revision = store.snapshot().revision();
    let prepared =
        writer.prepare_local_adjudication(&store, operation, request, revision, &head)?;
    let approved = approve_prepared(prepared.approval_bytes()?, prepared.approval_digest()?)?;
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
    operation: &'a str,
    graph: &'a census_store::GraphAuthorityReferenceV1,
    affected_record_keys: &'a [String],
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
#[derive(Serialize)]
struct SourceOutcome<'a> {
    schema: &'static str,
    operation: &'a str,
    subject: &'a str,
    public_id: &'a PublicAthleteId,
    head: &'a RegistryHead,
    replayed: bool,
    actor: &'static str,
    qualification_scope: &'static str,
}

pub(super) fn emit(value: &impl Serialize) -> Result<()> {
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
