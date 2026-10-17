use super::super::super::source_binding::SourceBindingObjectV1;
use super::*;
use fixture::{install, Fixture};

#[test]
fn controlled_bound_source_replay_matches_after_unrelated_live_store_write() -> TestResult {
    let fixture = Fixture::new()?;
    let capture = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&capture.athlete.id)?;
    issuance::review(&fixture, &capture, "controlled-replay")?;
    let dataset = fixture.dataset()?;
    let snapshot = fixture.store.snapshot();
    let objects = fixture.directory.path().join("replay-objects");
    std::fs::create_dir(&objects)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let athletes = crate::export::qualification::physical_observations::capture::<CanonicalAthlete>(
        &dataset,
        &snapshot,
        Table::Athletes,
        &objects,
        fixture::budget(),
    )?;
    let bound = BoundRosterSourceViewV1::capture(
        &dataset,
        &fixture.profile,
        &objects,
        &claims,
        athletes.clone(),
        fixture::instant()?,
        &mut grant,
    )?;
    let (pinned, family) = super::super::assess(&dataset, &fixture.profile, &bound, &mut grant)?;
    let expected = pinned.into_verified();
    let family_ref = SourceBindingObjectV1 {
        digest: family.digest().into(),
        bytes: u64::try_from(family.bytes().len())?,
    };
    let closure = bound.closure_reference().clone();
    fixture
        .store
        .journal_done("controlled-after-freeze", "later", &true)?;
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let replayed = BoundRosterSourceViewV1::replay(
        &dataset,
        &fixture.profile,
        &objects,
        &closure,
        &athletes,
        &family_ref,
        &mut replay_grant,
    )?;
    let (proof, replay_family) =
        super::super::assess(&dataset, &fixture.profile, &replayed, &mut replay_grant)?;
    let verified = proof.into_verified();
    check!(eq; verified.resolution(&capture.athlete.id), expected.resolution(&capture.athlete.id));
    check!(eq; verified.resolution(&capture.athlete.id).school(), Some(&capture.school.id));
    check!(eq; replay_family.bytes(), family.bytes());
    check!(eq; verified.audit(&capture.athlete.id).ok_or("replay audit")?.references(), expected.audit(&capture.athlete.id).ok_or("current audit")?.references());
    Ok(())
}

#[test]
fn controlled_source_adapter_refuses_foreign_grant_and_rebound_publication() -> TestResult {
    let fixture = Fixture::new()?;
    let capture = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&capture.athlete.id)?;
    issuance::review(&fixture, &capture, "controlled-foreign-grant")?;
    let dataset = fixture.dataset()?;
    let snapshot = fixture.store.snapshot();
    let objects = fixture.directory.path().join("grant-objects");
    std::fs::create_dir(&objects)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let athletes = crate::export::qualification::physical_observations::capture::<CanonicalAthlete>(
        &dataset,
        &snapshot,
        Table::Athletes,
        &objects,
        fixture::budget(),
    )?;
    let bound = BoundRosterSourceViewV1::capture(
        &dataset,
        &fixture.profile,
        &objects,
        &claims,
        athletes,
        fixture::instant()?,
        &mut grant,
    )?;
    let mut foreign = AttachmentReplayGrantV1::new();
    let result = super::super::assess(&dataset, &fixture.profile, &bound, &mut foreign);
    check!(matches!(result, Err(ReportError::Store(_))));
    check!(foreign.ensure_available().is_err());
    let (valid, _) = super::super::assess(&dataset, &fixture.profile, &bound, &mut grant)?;
    check!(eq; valid.into_verified().resolution(&capture.athlete.id).school(), Some(&capture.school.id));
    fixture
        .store
        .journal_done("controlled-new-publication", "new-input", &true)?;
    let changed = fixture.dataset()?;
    let result = super::super::assess(&changed, &fixture.profile, &bound, &mut grant);
    check!(matches!(result, Err(ReportError::Store(_))));
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn controlled_conflict_and_complete_claim_inventory_survive_frozen_replay() -> TestResult {
    let fixture = Fixture::new()?;
    let first = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&first.athlete.id)?;
    let second = install(&fixture, "12440", "42", Sport::OutdoorTrack, true)?;
    super::super::super::tests::identity::accept_pair(
        &fixture.store,
        &fixture.writer,
        &first.athlete.id,
        &second.athlete.id,
    )?;
    issuance::review(&fixture, &first, "controlled-frozen-first")?;
    issuance::review(&fixture, &second, "controlled-frozen-second")?;
    let dataset = fixture.dataset()?;
    let snapshot = fixture.store.snapshot();
    let objects = fixture.directory.path().join("conflict-objects");
    std::fs::create_dir(&objects)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let athletes = crate::export::qualification::physical_observations::capture::<CanonicalAthlete>(
        &dataset,
        &snapshot,
        Table::Athletes,
        &objects,
        fixture::budget(),
    )?;
    let bound = BoundRosterSourceViewV1::capture(
        &dataset,
        &fixture.profile,
        &objects,
        &claims,
        athletes.clone(),
        fixture::instant()?,
        &mut grant,
    )?;
    let (proof, family) = super::super::assess(&dataset, &fixture.profile, &bound, &mut grant)?;
    check!(eq; proof.into_verified().resolution(&first.athlete.id), &Resolution::Conflicted);
    let family_ref = SourceBindingObjectV1 {
        digest: family.digest().into(),
        bytes: u64::try_from(family.bytes().len())?,
    };
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let replayed = BoundRosterSourceViewV1::replay(
        &dataset,
        &fixture.profile,
        &objects,
        bound.closure_reference(),
        &athletes,
        &family_ref,
        &mut replay_grant,
    )?;
    let (proof, replay_family) =
        super::super::assess(&dataset, &fixture.profile, &replayed, &mut replay_grant)?;
    check!(eq; proof.into_verified().resolution(&first.athlete.id), &Resolution::Conflicted);
    check!(eq; replay_family.bytes(), family.bytes());
    Ok(())
}
