use anyhow::{ensure, Context, Result};
use census_domain::evidence_subject_attachment::{
    AttachmentActionV1, AttachmentDigestV1, RetainedEvidenceSubjectAttachmentCommandV1,
};
use census_store::{
    AttachmentInspectionRequestV1, EvidenceSubjectAttachmentPreparationV1,
    PreparedEvidenceSubjectAttachmentV1, Store,
};
use clap::Args;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};

mod materials;

#[derive(Debug, Args)]
pub(super) struct AttachmentReviewArgs {
    #[arg(long, value_name = "PATH")]
    command: PathBuf,
    #[arg(long, value_name = "PATH")]
    inline: Vec<PathBuf>,
    #[arg(long, value_name = "PATH")]
    retained_leaf: Vec<PathBuf>,
    #[arg(long, value_name = "SHA256")]
    approve_sha256: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct AttachmentReplayArgs {
    #[arg(long, value_name = "PATH")]
    command: PathBuf,
    #[arg(long, value_name = "SHA256")]
    approve_sha256: String,
}

fn plain_sha256(bytes: &[u8]) -> Result<AttachmentDigestV1> {
    Ok(AttachmentDigestV1::parse(&format!(
        "{:x}",
        Sha256::digest(bytes)
    ))?)
}

fn explicit_approval(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    value: &str,
) -> Result<AttachmentDigestV1> {
    let given = AttachmentDigestV1::parse(value).context("parsing the explicit approval SHA256")?;
    let bytes = command.approval_bytes(materials::COMMAND_MAX_BYTES)?;
    ensure!(
        given == plain_sha256(&bytes)?,
        "supplied approval SHA256 does not match the original command"
    );
    Ok(given)
}

fn inspection_request(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
) -> AttachmentInspectionRequestV1 {
    match &command.action {
        AttachmentActionV1::Attach { prior } => AttachmentInspectionRequestV1::Attach {
            target: command.target.public_id.clone(),
            prior: prior.clone(),
        },
        AttachmentActionV1::Revoke { prior } => AttachmentInspectionRequestV1::Revoke {
            prior: prior.clone(),
        },
    }
}

fn conclude<W: Write>(
    store: &Store,
    prepared: PreparedEvidenceSubjectAttachmentV1<'_>,
    given: Option<&AttachmentDigestV1>,
    replay: bool,
    output: &mut W,
) -> Result<()> {
    let Some(given) = given else {
        output.write_all(prepared.approval_bytes())?;
        return Ok(());
    };
    let plain = plain_sha256(prepared.approval_bytes())?;
    ensure!(
        given == &plain,
        "supplied approval SHA256 does not match preparation"
    );
    let mut approval = Vec::new();
    approval.try_reserve_exact(prepared.approval_bytes().len())?;
    approval.extend_from_slice(prepared.approval_bytes());
    let domain = prepared.approval_digest().clone();
    let operation = prepared.command().operation.as_str().to_owned();
    let applied = store
        .apply_evidence_subject_attachment_v1(prepared, &approval, &domain)
        .context("applying the exact reviewed attachment")?;
    ensure!(
        !replay || applied.repeated(),
        "attachment replay wrote a new record"
    );
    ensure!(
        store.receipt(&applied.receipt().operation)?.as_ref() == Some(applied.receipt()),
        "attachment reply differs from its durable receipt"
    );
    serde_json::to_writer_pretty(
        &mut *output,
        &serde_json::json!({
            "schema": "attachment-review-v1",
            "operation": operation,
            "approval_sha256": plain.as_str(),
            "approval_digest": domain.as_str(),
            "application": &applied,
            "appended": applied.appended(),
            "repeated": applied.repeated(),
            "receipt": applied.receipt(),
            "qualification_scope": "Explicit reviewed attachment only; no eligibility, current-school or calendar acceptance"
        }),
    )?;
    writeln!(output)?;
    Ok(())
}

pub(super) fn run_to<W: Write>(
    root: &Path,
    args: &AttachmentReviewArgs,
    output: &mut W,
) -> Result<()> {
    let command = materials::read_command(&args.command)?;
    let materials = materials::load(&command, &args.inline, &args.retained_leaf)?;
    let borrowed = materials.borrowed()?;
    let given = args
        .approve_sha256
        .as_deref()
        .map(|value| explicit_approval(&command, value))
        .transpose()?;
    let (store, _writer) = Store::open_current_qualification_v3(root)
        .context("opening the stopped current-qualification store")?
        .into_parts();
    let inspection = store
        .inspect_evidence_subject_attachment_v1(inspection_request(&command))
        .context("inspecting the exact attachment command")?;
    let prepared = store
        .prepare_evidence_subject_attachment_v1(
            command,
            EvidenceSubjectAttachmentPreparationV1::New {
                inspection,
                context: &borrowed,
            },
        )
        .context("preparing the exact attachment command")?;
    conclude(&store, prepared, given.as_ref(), false, output)
}

pub(super) fn replay_to<W: Write>(
    root: &Path,
    args: &AttachmentReplayArgs,
    output: &mut W,
) -> Result<()> {
    let command = materials::read_command(&args.command)?;
    let given = explicit_approval(&command, &args.approve_sha256)?;
    let (store, _writer) = Store::open_current_qualification_v3(root)
        .context("opening the stopped current-qualification store")?
        .into_parts();
    let prepared = store
        .prepare_evidence_subject_attachment_v1(
            command,
            EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
        )
        .context("preparing the original attachment replay")?;
    ensure!(
        prepared.is_replay(),
        "attachment replay did not find the original command"
    );
    conclude(&store, prepared, Some(&given), true, output)
}

#[cfg(test)]
mod tests;
