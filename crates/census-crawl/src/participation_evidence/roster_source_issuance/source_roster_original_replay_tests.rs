use super::*;
use census_store::RosterFrozenReviewedClaimsReferenceV1;

const ORIGINAL_BYTES: &[u8] = include_bytes!("source_roster_original_a/closure.json");
const ORIGINAL_INPUT: &[u8] = br#"{"version":2,"baseline_evidence_generation":0,"baseline_derived_generation":0,"counters":[0,1,0,0,0,0]}"#;

#[test]
fn original_nonempty_physical_closure_replays_after_current_capture_changes() -> TestResult {
    let reference: RosterFrozenReviewedClaimsReferenceV1 =
        serde_json::from_slice(include_bytes!("source_roster_original_a/reference.json"))?;
    check!(eq; hash(ORIGINAL_BYTES), "47bc00b12e8886de939e95b5f5ff8e89071ef0b1f03933d92664b7d957b27e07");
    let mut grant = AttachmentReplayGrantV1::new();
    let replay = replay_frozen_roster_reviewed_claims_v1(ORIGINAL_BYTES, &reference, &mut grant)?;
    check!(eq; replay.selected_snapshot_sequence(), 15);
    let claims = replay.claims(&mut grant)?;
    check!(eq; claims.len(), 1);
    let claim = claims.first().ok_or("original reviewed source absent")?;
    check!(eq; claim.original_input_token(&mut grant)?, ORIGINAL_INPUT);
    let (qualification, bytes) = claim.qualification_object(&mut grant)?;
    check!(eq; qualification, hash(bytes));
    let (activation, bytes) = claim.activation_object(&mut grant)?;
    check!(eq; activation, hash(bytes));

    let mut current_grant = AttachmentReplayGrantV1::new();
    let current =
        replay_frozen_roster_reviewed_claims_v1(ORIGINAL_BYTES, &reference, &mut current_grant)?;
    let current_claim = current
        .claims(&mut current_grant)?
        .first()
        .ok_or("original claim for current refusal")?;
    check!(matches!(
        assess_reviewed_roster_source_v1(current_claim, &mut current_grant),
        Err(crate::CrawlError::Store(
            census_store::StoreError::Refused { .. }
        ))
    ));
    check!(current_grant.ensure_available().is_err());

    let mut tampered = ORIGINAL_BYTES.to_vec();
    tampered.push(b' ');
    check!(replay_frozen_roster_reviewed_claims_v1(
        &tampered,
        &reference,
        &mut AttachmentReplayGrantV1::new(),
    )
    .is_err());
    Ok(())
}
