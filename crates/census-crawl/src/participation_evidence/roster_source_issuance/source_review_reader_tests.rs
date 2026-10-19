use super::*;
use crate::participation_evidence::roster_source_issuance::assess_reviewed_roster_source_v1;
use census_store::{
    replay_frozen_roster_reviewed_claims_v1, CurrentRosterReviewedClaimsV1,
    ReviewedRosterSourceClaimV1, Table,
};

use super::integration::setup;
use setup::*;

#[path = "source_frozen_canonical_tests.rs"]
mod canonical;

#[path = "source_frozen_encoding_tests.rs"]
mod compact_wire;

#[path = "source_roster_scale_tests.rs"]
mod scale;

#[path = "source_roster_original_replay_tests.rs"]
mod original_replay;

fn issued() -> TestResult<(tempfile::TempDir, census_store::Store)> {
    let (directory, store) = enrolled_store()?;
    let prepared = super::review_integration::prepare(&store, "controlled-reader", None)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    store.apply_roster_source_review_v1(prepared, &approval, &digest)?;
    Ok((directory, store))
}

fn reassess(
    claim: &ReviewedRosterSourceClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    let assessed = assess_reviewed_roster_source_v1(claim, grant)?;
    let proposal = assessed.proposal(grant)?;
    check!(eq; &assessed.school(grant)?.id, proposal.binding().school());
    check!(eq; &assessed.program(grant)?.id, proposal.binding().program());
    assessed.source(grant)?.retained_packet(grant)?;
    let (qualification, bytes) = assessed.qualification_object(grant)?;
    check!(eq; qualification, hash(bytes));
    let (activation, bytes) = assessed.activation_object(grant)?;
    check!(eq; activation, hash(bytes));
    check!(qualification != activation);
    Ok(())
}

fn first(
    current: &CurrentRosterReviewedClaimsV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
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
    let replay = replay_frozen_roster_reviewed_claims_v1(
        frozen.bytes(),
        frozen.reference(),
        &mut replay_grant,
    )?;
    check!(eq; replay.selected_snapshot_sequence(), snapshot.sequence());
    reassess(
        replay
            .claims(&mut replay_grant)?
            .first()
            .ok_or("replayed review absent")?,
        &mut replay_grant,
    )?;
    let mut wrong = serde_json::to_value(frozen.reference())?;
    wrong["digest"] = json!("d".repeat(64));
    let wrong = serde_json::from_value(wrong)?;
    check!(replay_frozen_roster_reviewed_claims_v1(
        frozen.bytes(),
        &wrong,
        &mut AttachmentReplayGrantV1::new()
    )
    .is_err());
    let mut changed = frozen.bytes().to_vec();
    changed.push(b' ');
    check!(replay_frozen_roster_reviewed_claims_v1(
        &changed,
        frozen.reference(),
        &mut AttachmentReplayGrantV1::new()
    )
    .is_err());
    check!(current.claims(&mut AttachmentReplayGrantV1::new()).is_err());
    Ok(())
}

#[test]
fn school_correction_preserves_original_review_and_store_free_replay() -> TestResult {
    let (directory, store) = issued()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    let original_input = current
        .claims(&mut grant)?
        .first()
        .ok_or("original review absent")?
        .original_input_token(&mut grant)?
        .to_vec();
    let frozen = current.freeze_closure(&mut grant)?;
    let (_, mut school, _, _) = install(&store)?;
    let mut repeated_grant = AttachmentReplayGrantV1::new();
    let repeated = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut repeated_grant)?;
    first(&repeated, &mut repeated_grant)?;
    school.name = "Controlled Renamed School".into();
    store.append(Table::Schools, &school)?;
    let mut corrected_grant = AttachmentReplayGrantV1::new();
    let corrected = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut corrected_grant)?;
    first(&corrected, &mut corrected_grant)?;
    check!(eq; original_input.as_slice(), corrected
        .claims(&mut corrected_grant)?
        .first().ok_or("corrected review absent")?
        .original_input_token(&mut corrected_grant)?);
    let corrected_frozen = corrected.freeze_closure(&mut corrected_grant)?;
    check!(
        frozen.reference().current_input_token()
            != corrected_frozen.reference().current_input_token()
    );
    drop(store);
    directory.close()?;
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let replay = replay_frozen_roster_reviewed_claims_v1(
        frozen.bytes(),
        frozen.reference(),
        &mut replay_grant,
    )?;
    let claim = replay
        .claims(&mut replay_grant)?
        .first()
        .ok_or("replayed review absent")?;
    check!(eq; original_input.as_slice(), claim.original_input_token(&mut replay_grant)?);
    reassess(claim, &mut replay_grant)?;
    Ok(())
}

#[test]
fn rehashed_frozen_selected_metadata_requires_original_publication_binding() -> TestResult {
    let (_directory, store) = issued()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    let frozen = current.freeze_closure(&mut grant)?;
    for field in ["selected_snapshot_sequence", "current_input_token"] {
        let mut altered: Value = serde_json::from_slice(frozen.bytes())?;
        if field == "selected_snapshot_sequence" {
            altered[field] = json!(current
                .selected_snapshot_sequence()
                .checked_add(1)
                .ok_or("sequence overflow")?);
        } else {
            altered[field] = json!([123, 125]);
        }
        let bytes = serde_json::to_vec(&altered)?;
        let mut expected = serde_json::to_value(frozen.reference())?;
        expected["digest"] = json!(hash(&bytes));
        expected["bytes"] = json!(bytes.len());
        let expected = serde_json::from_value(expected)?;
        check!(replay_frozen_roster_reviewed_claims_v1(
            &bytes,
            &expected,
            &mut AttachmentReplayGrantV1::new()
        )
        .is_err());
    }
    Ok(())
}

#[test]
fn combined_semantic_capability_rejects_foreign_and_poisoned_grants() -> TestResult {
    let (_directory, store) = issued()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    let claim = current.claims(&mut grant)?.first().ok_or("review absent")?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(assess_reviewed_roster_source_v1(claim, &mut foreign).is_err());
    let assessed = assess_reviewed_roster_source_v1(claim, &mut grant)?;
    check!(assessed
        .source(&mut AttachmentReplayGrantV1::new())
        .is_err());
    grant.poison();
    check!(assessed.qualification_object(&mut grant).is_err());
    check!(assessed.activation_object(&mut grant).is_err());
    check!(assessed.school(&mut grant).is_err());
    Ok(())
}
