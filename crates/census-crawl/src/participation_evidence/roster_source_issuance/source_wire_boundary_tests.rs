use super::*;
use census_store::{
    replay_frozen_roster_reviewed_claims_v1, RosterFrozenReviewedClaimsReferenceV1,
    RosterSourceIssuanceStoreError, StoreError,
};
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn wire_boundary_at_size(bytes: usize, operation: &str) -> TestResult {
    let (directory, store) = enrolled_store()?;
    let extra = vec![b'x'; bytes];
    let extra_digest = hash(&extra);
    let prepared = prepare(&store, operation, Some(bytes))?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let applied = store.apply_roster_source_review_v1(prepared, &approved, &digest)?;
    check!(applied.written());
    let snapshot = store.snapshot();
    let mut original_grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_roster_reviewed_claims_v1(&mut original_grant)?;
    let claims = current.claims(&mut original_grant)?;
    check!(eq; claims.len(), 1);
    let claim = claims.first().ok_or("accepted syntactic review claim")?;
    let object = claim.object(&extra_digest, &mut original_grant)?;
    check!(eq; object, &extra);
    let frozen = current.freeze_closure(&mut original_grant)?;
    drop(current);
    drop(snapshot);
    drop(store);
    directory.close()?;
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let replay = replay_frozen_roster_reviewed_claims_v1(
        frozen.bytes(),
        frozen.reference(),
        &mut replay_grant,
    )?;
    let replay_claims = replay.claims(&mut replay_grant)?;
    check!(eq; replay_claims.len(), 1);
    let replay_claim = replay_claims
        .first()
        .ok_or("replayed review claim absent")?;
    let replay_object = replay_claim.object(&extra_digest, &mut replay_grant)?;
    check!(eq; replay_object, &extra);
    Ok(())
}

#[test]
fn wire_boundary_at_three_mib() -> TestResult {
    wire_boundary_at_size(3 * 1024 * 1024, "wire-boundary-3mib")
}

#[test]
fn wire_boundary_at_three_mib_plus_one() -> TestResult {
    wire_boundary_at_size(3 * 1024 * 1024 + 1, "wire-boundary-3mib+1")
}

#[test]
fn wire_boundary_at_four_mib() -> TestResult {
    wire_boundary_at_size(4 * 1024 * 1024, "wire-boundary-4mib")
}

#[test]
fn wire_boundary_at_four_mib_plus_one_refuses() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let error = prepare(&store, "wire-boundary-4mib+1", Some(4 * 1024 * 1024 + 1))
        .err()
        .ok_or("prepare must reject a 4MiB+1 extra object")?;
    check!(matches!(
        error.downcast_ref::<StoreError>().map(|e| matches!(
            e,
            StoreError::RosterSourceIssuance(RosterSourceIssuanceStoreError::BudgetExceeded)
        )),
        Some(true)
    ));
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let claims = current.claims(&mut grant)?;
    check!(claims.is_empty());
    Ok(())
}

type PreparedFrozenBundle = (
    tempfile::TempDir,
    Store,
    Vec<u8>,
    RosterFrozenReviewedClaimsReferenceV1,
    Vec<u8>,
);

fn prepared_frozen_bundle() -> TestResult<PreparedFrozenBundle> {
    let (directory, store) = enrolled_store()?;
    let extra = vec![b'x'; 256];
    let extra_digest = hash(&extra);
    let prepared = prepare(&store, "frozen-ceiling", Some(256))?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let applied = store.apply_roster_source_review_v1(prepared, &approved, &digest)?;
    check!(applied.written());
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let claims = current.claims(&mut grant)?;
    check!(eq; claims.len(), 1);
    let object = claims
        .first()
        .ok_or("accepted claim")?
        .object(&extra_digest, &mut grant)?;
    check!(eq; object, &extra);
    let frozen = current.freeze_closure(&mut grant)?;
    let frozen_bytes = frozen.bytes().to_vec();
    let reference = frozen.reference().clone();
    Ok((directory, store, frozen_bytes, reference, extra))
}

#[test]
fn frozen_bundle_ceiling_refuses_oversized_envelope() -> TestResult {
    let (directory, store, frozen_bytes, reference, original_extra) = prepared_frozen_bundle()?;
    drop(store);
    drop(directory);
    let ceiling = RosterFrozenReviewedClaimsReferenceV1::MAX_ENCODED_BYTES;
    let oversized = ceiling.checked_add(1).ok_or("ceiling overflow")?;
    let invalid_envelope = vec![b'o'; oversized];
    let mut poisoned_grant = AttachmentReplayGrantV1::new();
    let error =
        replay_frozen_roster_reviewed_claims_v1(&invalid_envelope, &reference, &mut poisoned_grant)
            .err()
            .ok_or("replay must reject oversized envelope")?;
    check!(matches!(
        error,
        StoreError::RosterSourceIssuance(RosterSourceIssuanceStoreError::BudgetExceeded)
    ));
    let reused_error =
        replay_frozen_roster_reviewed_claims_v1(&frozen_bytes, &reference, &mut poisoned_grant)
            .err()
            .ok_or("reusing poisoned grant must still fail")?;
    check!(matches!(
        reused_error,
        StoreError::EvidenceSubjectAttachment(
            census_store::EvidenceSubjectAttachmentStoreError::Domain(
                census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned
            )
        )
    ));
    let mut fresh_grant = AttachmentReplayGrantV1::new();
    let replayed =
        replay_frozen_roster_reviewed_claims_v1(&frozen_bytes, &reference, &mut fresh_grant)?;
    let replayed_claims = replayed.claims(&mut fresh_grant)?;
    check!(eq; replayed_claims.len(), 1);
    let replayed_claim = replayed_claims.first().ok_or("replayed claim absent")?;
    let replayed_object = replayed_claim.object(&hash(&original_extra), &mut fresh_grant)?;
    check!(eq; replayed_object, &original_extra);
    Ok(())
}
