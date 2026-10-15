use anyhow::{Context, Result};
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdV1;
use census_domain::public_identity_history::RegistryHead;
use census_store::MembershipWithholdingOutcomeV1;
use serde::Serialize;
use std::path::Path;

pub(super) fn withhold(root: &Path, operation: &str, path: &Path) -> Result<()> {
    let bytes = census_crawl::convert::read_file_capped(path, 1024 * 1024)?;
    let request: LocalMembershipWithholdV1 =
        serde_json::from_slice(&bytes).context("decoding exact membership withholding request")?;
    let (store, writer) = super::command::open(root)?;
    let head = writer.head(&store)?;
    let prepared = writer.prepare_membership_withholding(
        &store,
        operation,
        request,
        store.snapshot().revision(),
        &head,
    )?;
    let approved =
        super::approve_prepared(prepared.approval_bytes()?, prepared.approval_digest()?)?;
    let (bytes, digest) = approved.into_approved_record();
    let outcome = writer.apply_membership_withholding(&store, prepared, bytes, digest.clone())?;
    emit(operation, &digest, &outcome).context("membership withholding committed; if output failed use membership-receipt with the approved digest")
}

pub(super) fn receipt(root: &Path, operation: &str, approval_digest: &str) -> Result<()> {
    let (store, writer) = super::command::open(root)?;
    let outcome = writer
        .replay_membership_withholding(&store, operation, approval_digest)?
        .context("no membership withholding receipt for this operation")?;
    emit(operation, approval_digest, &outcome)
}

#[derive(Serialize)]
struct Outcome<'a> {
    schema: &'static str,
    operation: &'a str,
    approval_digest: &'a str,
    head: &'a RegistryHead,
    withheld_memberships: &'a [String],
    replayed: bool,
}
fn emit(
    operation: &str,
    approval_digest: &str,
    outcome: &MembershipWithholdingOutcomeV1,
) -> Result<()> {
    super::command::emit(&Outcome {
        schema: "local-membership-withholding-outcome/v1",
        operation,
        approval_digest,
        head: outcome.head(),
        withheld_memberships: outcome.withheld_memberships(),
        replayed: outcome.replayed(),
    })
}

pub(super) fn preview(
    root: &Path,
    operation: &str,
    canonical: &census_domain::public_identity_history::PublicAthleteId,
    sequence: u64,
    digest: &str,
    membership_id: &str,
) -> Result<()> {
    let (store, writer) = super::command::open(root)?;
    let attachment = census_domain::public_identity_history::local_membership_withholding::MembershipAttachmentV1 {
        sequence, digest: digest.to_owned(), membership_id: membership_id.to_owned(),
    };
    let expected_withheld_memberships =
        writer.preview_membership_withholding(&store, canonical, &attachment)?;
    let inventory = writer.inspect_membership_withholding(&store, operation, canonical)?;
    super::command::emit(&serde_json::json!({
        "schema": "local-membership-withholding-preview/v2", "canonical": canonical,
        "operation": operation, "graph": inventory.graph(),
        "head": writer.head(&store)?, "attachment": attachment,
        "expected_withheld_memberships": expected_withheld_memberships,
        "affected_record_keys": inventory.affected_record_keys(),
    }))
}
