use super::super::tests::{
    fixture::{self, Fixture},
    identity,
};
use super::*;
use census_domain::model::{serialized_digest, CanonicalAthlete};
use census_store::Table;

mod parity;
mod physical;
mod root;
mod tamper;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn objects(directory: &tempfile::TempDir) -> TestResult<std::path::PathBuf> {
    let path = directory.path().join("objects");
    std::fs::create_dir(&path)?;
    Ok(path)
}

#[test]
fn physical_snapshot_freezes_and_replays_after_live_archives_and_store_change() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected = snapshot.tables_digest(&[Table::Athletes])?;
    let (descriptor, proof) = capture(&dataset, &snapshot, &objects)?;
    let descriptor_digest = descriptor.digest()?;
    let original = proof.into_verified();
    let identities = snapshot.freeze_public_identity_v2()?;
    let frozen_dataset = directory.path().join("dataset.json");
    dataset.save_frozen(&frozen_dataset)?;
    let mut reopened = ExportDataset::reopen_frozen(&frozen_dataset)?;
    reopened.bind_public_identity_fixture_from_accepted(identities.replay()?)?;
    std::fs::remove_file(&fixture.body_path)?;
    std::fs::remove_file(&fixture.metadata_path)?;
    fixture::append_roster(
        &fixture.store,
        "26848",
        "Later High",
        "2026-10-08T12:00:00Z",
    )?;
    fixture
        .store
        .journal_done("after-affiliation-freeze", "changed", &true)?;
    let replayed = replay(
        &reopened,
        &objects,
        &descriptor,
        &expected,
        &descriptor_digest,
    )?
    .into_verified();
    check!(eq; replayed.resolution(&fixture.athlete), original.resolution(&fixture.athlete));
    check!(eq; replayed.audit(&fixture.athlete).ok_or("replay audit")?.references(), original.audit(&fixture.athlete).ok_or("source audit")?.references());
    check!(eq; descriptor.snapshot_sequence, snapshot.sequence());
    check!(fixture.store.snapshot().sequence() > descriptor.snapshot_sequence);
    check!(eq; serialized_digest(&reopened)?, serialized_digest(&dataset)?);
    Ok(())
}

#[test]
fn frozen_unknown_does_not_gain_authority_when_live_capture_becomes_available() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let original_body = std::fs::read(&fixture.body_path)?;
    std::fs::remove_file(&fixture.body_path)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected = snapshot.tables_digest(&[Table::Athletes])?;
    let (descriptor, proof) = capture(&dataset, &snapshot, &objects)?;
    let descriptor_digest = descriptor.digest()?;
    check!(eq; proof.into_verified().resolution(&fixture.athlete), &super::super::CurrentAffiliationResolution::Unknown);
    std::fs::write(&fixture.body_path, &original_body)?;
    proof_objects::install(&objects, &original_body)?;
    let resolved_live = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    check!(eq; resolved_live.resolution(&fixture.athlete).school(), Some(&fixture.school));
    let replayed = replay(
        &dataset,
        &objects,
        &descriptor,
        &expected,
        &descriptor_digest,
    )?
    .into_verified();
    check!(eq; replayed.resolution(&fixture.athlete), &super::super::CurrentAffiliationResolution::Unknown);
    let audit = replayed.audit(&fixture.athlete).ok_or("unknown audit")?;
    check!(audit.references().is_empty());
    check!(eq; audit.unavailable(), &["archive_unavailable".to_string()]);
    Ok(())
}

#[test]
fn frozen_conflict_retains_both_capture_proofs_after_live_archive_removal() -> TestResult {
    let fixture = Fixture::new("2026-10-07T09:00:00Z", "2026-10-07", true)?;
    let (other, _, other_body, other_metadata) = fixture::append_roster(
        &fixture.store,
        "26848",
        "Current High",
        "2026-10-07T12:00:00Z",
    )?;
    let canonical =
        identity::accept_pair(&fixture.store, &fixture.writer, &fixture.athlete, &other)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected = snapshot.tables_digest(&[Table::Athletes])?;
    let (descriptor, _) = capture(&dataset, &snapshot, &objects)?;
    let descriptor_digest = descriptor.digest()?;
    let identities = snapshot.freeze_public_identity_v2()?;
    let frozen_dataset = directory.path().join("conflicting-dataset.json");
    dataset.save_frozen(&frozen_dataset)?;
    let mut reopened = ExportDataset::reopen_frozen(&frozen_dataset)?;
    reopened.bind_public_identity_fixture_from_accepted(identities.replay()?)?;
    for path in [
        &fixture.body_path,
        &fixture.metadata_path,
        &other_body,
        &other_metadata,
    ] {
        std::fs::remove_file(path)?;
    }
    let replayed = replay(
        &reopened,
        &objects,
        &descriptor,
        &expected,
        &descriptor_digest,
    )?
    .into_verified();
    check!(eq; replayed.resolution(&canonical), &super::super::CurrentAffiliationResolution::Conflicted);
    let audit = replayed.audit(&canonical).ok_or("conflict audit")?;
    check!(eq; audit.references().len(), 4);
    check!(audit.unavailable().is_empty());
    check!(eq; audit.references().iter().filter(|reference| reference.contains("#capture=")).count(), 2);
    identity::assert_unselected(&reopened, &replayed, &canonical)?;
    check!(eq; serialized_digest(&reopened)?, serialized_digest(&dataset)?);
    Ok(())
}

#[test]
fn frozen_physical_stream_keeps_repeated_subject_observations() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let athlete = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .next()
        .ok_or("athlete")?;
    fixture.store.append(Table::Athletes, &athlete)?;
    fixture::accept(&fixture.store, &fixture.writer, &fixture.athlete)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected = snapshot.tables_digest(&[Table::Athletes])?;
    let (descriptor, _) = capture(&dataset, &snapshot, &objects)?;
    let descriptor_digest = descriptor.digest()?;
    check!(eq; descriptor.observations.counts().key_visits, 2);
    let replayed = replay(
        &dataset,
        &objects,
        &descriptor,
        &expected,
        &descriptor_digest,
    )?
    .into_verified();
    check!(eq; replayed.resolution(&fixture.athlete).school(), Some(&fixture.school));
    check!(eq; replayed.audit(&fixture.athlete).ok_or("deduplicated audit")?.references().len(), 2);
    Ok(())
}
