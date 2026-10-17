use super::*;
use crate::participation_evidence::roster_source_issuance::{assess_existing_roster_binding, policies::RosterInterpretationContractsV1};
use census_store::{CurrentRosterReviewedClaimsV1, ReviewedRosterSourceClaimV1, replay_frozen_roster_reviewed_claims_v1};

#[path = "source_integration_setup.rs"]
mod setup;
use setup::*;

fn issued() -> TestResult<(tempfile::TempDir, census_store::Store)> {
    let (directory, store) = enrolled_store()?;
    let prepared = super::review_integration::prepare(&store, "controlled-reader", None)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    store.apply_roster_source_review_v1(prepared, &approval, &digest)?;
    Ok((directory, store))
}

fn reassess(claim: &ReviewedRosterSourceClaimV1, grant: &mut AttachmentReplayGrantV1) -> TestResult {
    let proposal = claim.proposal(grant)?;
    let parser = claim.object(proposal.parser().artifact().digest().as_str(), grant)?;
    let contracts = RosterInterpretationContractsV1 {
        scope: claim.object(proposal.scope().policy().digest(), grant)?,
        year: claim.object(proposal.binding().year_convention().digest(), grant)?,
        projection_merge: claim.object(proposal.binding().projection_merge_contract().digest(), grant)?,
    };
    let source = assess_retained_roster_source(claim.evidence(grant)?, parser, &contracts, proposal, grant)?;
    let binding = assess_existing_roster_binding(&source, grant)?;
    check!(eq; &binding.school(grant)?.id, proposal.binding().school());
    check!(eq; &binding.program(grant)?.id, proposal.binding().program());
    let (qualification, bytes) = claim.qualification_object(grant)?;
    check!(eq; qualification, hash(bytes));
    let (activation, bytes) = claim.activation_object(grant)?;
    check!(eq; activation, hash(bytes));
    check!(qualification != activation);
    Ok(())
}

fn first(current: &CurrentRosterReviewedClaimsV1, grant: &mut AttachmentReplayGrantV1) -> TestResult {
    check!(eq; current.source_store_lifecycle(), 8);
    check!(eq; current.admission_policy(), "exact-inspected-closure");
    let claims = current.claims(grant)?;
    check!(eq; claims.len(), 1);
    reassess(claims.first().ok_or("review claim absent")?, grant)
}

#[test]
fn controlled_current_and_frozen_review_rederive_same_source() -> TestResult {
    let (_directory, store) = issued()?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    check!(eq; current.selected_snapshot_sequence(), snapshot.sequence());
    first(&current, &mut grant)?;
    let frozen = current.freeze_closure(&mut grant)?;
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let replay = replay_frozen_roster_reviewed_claims_v1(frozen.bytes(), frozen.reference(), &mut replay_grant)?;
    check!(eq; replay.selected_snapshot_sequence(), snapshot.sequence());
    reassess(replay.claims(&mut replay_grant)?.first().ok_or("replayed review absent")?, &mut replay_grant)?;
    let mut wrong = serde_json::to_value(frozen.reference())?;
    wrong["digest"] = json!("d".repeat(64));
    let wrong = serde_json::from_value(wrong)?;
    check!(replay_frozen_roster_reviewed_claims_v1(frozen.bytes(), &wrong, &mut AttachmentReplayGrantV1::new()).is_err());
    let mut changed = frozen.bytes().to_vec();
    changed.push(b' ');
    check!(replay_frozen_roster_reviewed_claims_v1(&changed, frozen.reference(), &mut AttachmentReplayGrantV1::new()).is_err());
    check!(current.claims(&mut AttachmentReplayGrantV1::new()).is_err());
    Ok(())
}

#[test]
fn physical_change_refuses_current_but_preserves_selected_frozen_review() -> TestResult {
    let (_directory, store) = issued()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = store.snapshot().read_current_roster_reviewed_claims_v1(&mut grant)?;
    let frozen = current.freeze_closure(&mut grant)?;
    install(&store)?;
    let result = store.snapshot().read_current_roster_reviewed_claims_v1(&mut AttachmentReplayGrantV1::new());
    check!(matches!(result, Err(census_store::StoreError::RosterSourceIssuance(census_store::RosterSourceIssuanceStoreError::InspectedClosureChanged))));
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let replay = replay_frozen_roster_reviewed_claims_v1(frozen.bytes(), frozen.reference(), &mut replay_grant)?;
    reassess(replay.claims(&mut replay_grant)?.first().ok_or("replayed review absent")?, &mut replay_grant)?;
    Ok(())
}

#[test]
fn rehashed_frozen_selected_metadata_requires_original_publication_binding() -> TestResult {
    let (_directory, store) = issued()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = store.snapshot().read_current_roster_reviewed_claims_v1(&mut grant)?;
    let frozen = current.freeze_closure(&mut grant)?;
    for field in ["selected_snapshot_sequence", "current_input_token"] {
        let mut altered: Value = serde_json::from_slice(frozen.bytes())?;
        if field == "selected_snapshot_sequence" {
            altered[field] = json!(current.selected_snapshot_sequence().checked_add(1).ok_or("sequence overflow")?);
        } else { altered[field] = json!([123, 125]); }
        let bytes = serde_json::to_vec(&altered)?;
        let mut expected = serde_json::to_value(frozen.reference())?;
        expected["digest"] = json!(hash(&bytes));
        expected["bytes"] = json!(bytes.len());
        let expected = serde_json::from_value(expected)?;
        check!(replay_frozen_roster_reviewed_claims_v1(&bytes, &expected, &mut AttachmentReplayGrantV1::new()).is_err());
    }
    Ok(())
}
