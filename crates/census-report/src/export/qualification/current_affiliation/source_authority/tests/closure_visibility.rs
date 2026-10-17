use super::*;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;
use census_store::{
    replay_frozen_roster_reviewed_claims_v1, ReviewedRosterSourceClaimV1,
    RosterSourceIssuanceStoreError,
};

fn isolated(
    claims: &[ReviewedRosterSourceClaimV1],
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    let first = claims.first().ok_or("first reviewed claim absent")?;
    let second = claims.get(1).ok_or("second reviewed claim absent")?;
    let (first_digest, first_bytes) = first.qualification_object(grant)?;
    let (second_digest, second_bytes) = second.qualification_object(grant)?;
    check!(first_digest != second_digest);
    check!(eq; first.object(first_digest, grant)?, first_bytes);
    check!(eq; second.object(second_digest, grant)?, second_bytes);
    for (claim, forbidden) in [(first, second_digest), (second, first_digest)] {
        check!(matches!(
            claim.object(forbidden, grant),
            Err(census_store::StoreError::RosterSourceIssuance(
                RosterSourceIssuanceStoreError::Corrupt("review source object absent")
            ))
        ));
    }
    Ok(())
}

#[test]
fn per_claim_closure_prevents_foreign_object_access() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let first = fixture::install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&first.athlete.id)?;
    let second = fixture::install(&fixture, "12440", "42", Sport::OutdoorTrack, true)?;
    super::super::super::tests::identity::accept_pair(
        &fixture.store,
        &fixture.writer,
        &first.athlete.id,
        &second.athlete.id,
    )?;
    issuance::review(&fixture, &first, "closure-visibility-first")?;
    issuance::review(&fixture, &second, "closure-visibility-second")?;
    let bundle = {
        let snapshot = fixture.store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
        isolated(current.claims(&mut grant)?, &mut grant)?;
        current.freeze_closure(&mut grant)?
    };
    drop(fixture);
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let replay = replay_frozen_roster_reviewed_claims_v1(
        bundle.bytes(),
        bundle.reference(),
        &mut replay_grant,
    )?;
    isolated(replay.claims(&mut replay_grant)?, &mut replay_grant)
}
#[test]
fn replayed_claim_rejects_foreign_grant_and_accepts_original() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let first = fixture::install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&first.athlete.id)?;
    issuance::review(&fixture, &first, "replay-foreign-grant")?;
    let bundle = {
        let snapshot = fixture.store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
        current.freeze_closure(&mut grant)?
    };
    drop(fixture);
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let replay = replay_frozen_roster_reviewed_claims_v1(
        bundle.bytes(),
        bundle.reference(),
        &mut replay_grant,
    )?;
    let claims = replay.claims(&mut replay_grant)?;
    let claim = claims.first().ok_or("replayed claim absent")?;
    let (digest, expected_bytes) = claim.qualification_object(&mut replay_grant)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    let usage = replay_grant.usage();
    foreign.charge_bytes(usage.encoded_bytes())?;
    foreign.charge_steps(usage.traversal_steps())?;
    let _foreign_origin = foreign.checkpoint()?;
    let foreign_err = claim.qualification_object(&mut foreign);
    check!(matches!(
        foreign_err,
        Err(census_store::StoreError::EvidenceSubjectAttachment(
            census_store::EvidenceSubjectAttachmentStoreError::Domain(
                EvidenceSubjectAttachmentError::Invalid(_)
            )
        ))
    ));
    let foreign_err = claim.object(digest, &mut foreign);
    check!(matches!(
        foreign_err,
        Err(census_store::StoreError::EvidenceSubjectAttachment(
            census_store::EvidenceSubjectAttachmentStoreError::Domain(
                EvidenceSubjectAttachmentError::Poisoned
            )
        ))
    ));
    let own = claim.object(digest, &mut replay_grant)?;
    check!(eq; own, expected_bytes);
    Ok(())
}
