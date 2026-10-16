use super::reviewed_fixture::{binding_id, ready_for_manual as ready_for_merge, ReviewedFixture};
use crate::public_identity::{local_adjudication, PublicIdentityError};
use crate::public_identity_tests::support::TestResult;
use crate::StoreError;
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, LocalIdentitySplitV1, RetainedClaimDispositionV1, SupersededMergeV1,
};
use census_domain::public_identity_history::{BindingPartition, PublicResolution};
use serde::Serialize;
use sha2::{Digest, Sha256};

fn request(fixture: &ReviewedFixture) -> TestResult<LocalIdentitySplitV1> {
    let canonical = fixture.births.first().ok_or("first birth")?.public_id();
    let head = fixture.writer.head(&fixture.store)?;
    let inventory = fixture
        .writer
        .inspect_local_adjudication(&fixture.store, canonical)?;
    let dispositions = inventory
        .claims()
        .iter()
        .map(|claim| RetainedClaimDispositionV1 {
            claim: claim.clone(),
            treatment: if claim.claim_path.starts_with("/source_links/")
                || claim.claim_path.starts_with("/identity_attestations/")
            {
                ClaimTreatmentV1::RejectIdentityCorroboration
            } else {
                ClaimTreatmentV1::RetainAsContext
            },
            rationale:
                "Synthetic local review: published link combined two separately retained athletes"
                    .into(),
        })
        .collect();
    let partitions = fixture
        .candidates
        .iter()
        .zip(&fixture.births)
        .take(2)
        .map(|(candidate, birth)| {
            Ok(BindingPartition {
                representative: candidate.clone(),
                binding_ids: vec![binding_id(&fixture.store, birth.head().sequence)?],
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(LocalIdentitySplitV1 {
        case_id: fixture.request.case_id().into(), canonical: canonical.clone(),
        supersedes: vec![SupersededMergeV1 { sequence: head.sequence, digest: head.digest }],
        partitions, unresolved_bindings: Vec::new(),
        affected_record_keys: inventory.affected_record_keys().to_vec(), dispositions,
        rationale: "Synthetic reviewed correction retains original source facts and reverses their identity interpretation".into(),
    })
}

fn merged() -> TestResult<ReviewedFixture> {
    let fixture = ready_for_merge()?;
    results::install(&fixture)?;
    fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "synthetic-manual-prior-merge",
        &fixture.request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    Ok(fixture)
}

fn approval(
    prepared: &local_adjudication::PreparedLocalAdjudicationV1<'_>,
) -> TestResult<(Vec<u8>, String)> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        schema: &'static str,
        actor: &'static str,
        prepared: &'a census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2,
    }
    let bytes = serde_json::to_vec(&Envelope {
        schema: "local-identity-adjudication-approval/v1",
        actor: "local_operator",
        prepared: prepared.record(),
    })?;
    let mut digest = Sha256::new();
    digest.update(b"local-identity-adjudication-approval/v1");
    digest.update([0]);
    digest.update(&bytes);
    Ok((bytes, format!("{:x}", digest.finalize())))
}

#[test]
fn genuine_merge_can_prepare_and_consume_explicit_manual_split_without_erasing_claims() -> TestResult
{
    let fixture = merged()?;
    let before = super::Inputs::capture(&fixture.store.snapshot())?.athletes;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-split",
        request,
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let approved = approval(&prepared)?;
    let old = fixture.births.first().ok_or("birth")?.public_id();
    local_adjudication::consume(
        &fixture.writer,
        &fixture.store,
        prepared,
        approved,
        |_, approved, ledger| {
            let projection = ledger
                .history
                .projection()
                .clone()
                .extend_candidate(&approved.history.uncommitted_record)
                .map_err(PublicIdentityError::from)?;
            assert!(matches!(
                projection
                    .resolve_public(old)
                    .map_err(PublicIdentityError::from)?,
                PublicResolution::Ambiguous { .. }
            ));
            Ok(())
        },
    )?;
    let after = super::Inputs::capture(&fixture.store.snapshot())?.athletes;
    assert_eq!(serde_json::to_value(before)?, serde_json::to_value(after)?);
    assert_eq!(fixture.writer.head(&fixture.store)?, head);
    Ok(())
}

#[test]
fn omitted_retained_claim_cannot_prepare_a_manual_correction() -> TestResult {
    let fixture = merged()?;
    let mut request = request(&fixture)?;
    request.dispositions.pop().ok_or("claim")?;
    let result = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-omission",
        request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    );
    assert!(result.is_err());
    Ok(())
}

#[test]
fn changed_approval_bytes_never_reach_the_atomic_commit_hook() -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-corrupt",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let mut approved = approval(&prepared)?;
    approved.0.push(b' ');
    let mut reached = false;
    let result = local_adjudication::consume(
        &fixture.writer,
        &fixture.store,
        prepared,
        approved,
        |_, _, _| {
            reached = true;
            Ok(())
        },
    );
    assert!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual approval binding"
        )))
    ));
    assert!(!reached);
    Ok(())
}

#[test]
fn changed_store_revision_after_approval_cannot_reach_commit() -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-stale",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let approved = approval(&prepared)?;
    let extra = crate::public_identity_tests::support::subject("manual-new-evidence", "9991")?;
    fixture
        .store
        .append_many(crate::Table::Athletes, &[extra])?;
    let mut reached = false;
    let result = local_adjudication::consume(
        &fixture.writer,
        &fixture.store,
        prepared,
        approved,
        |_, _, _| {
            reached = true;
            Ok(())
        },
    );
    assert!(matches!(result, Err(StoreError::RevisionMoved { .. })));
    assert!(!reached);
    Ok(())
}

#[test]
fn forged_retained_claim_value_cannot_prepare_even_when_its_digest_is_recomputed() -> TestResult {
    let fixture = merged()?;
    let mut request = request(&fixture)?;
    let claim = &mut request.dispositions.first_mut().ok_or("claim")?.claim;
    claim.retained_value = serde_json::json!("invented fact");
    claim.claim_digest = census_domain::model::serialized_digest(&claim.retained_value)?;
    let result = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-forged",
        request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    );
    assert!(matches!(result, Err(StoreError::PublicIdentity(_))));
    Ok(())
}

mod affected;
mod affected_replay;
mod durable;
mod frozen_refusals;
mod results;
mod retained_validation;

mod admission;
