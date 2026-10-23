use crate::export::{CurrentAffiliationResolution, ExportDataset, ParticipationOutcome};
use census_store::{Store, Table};
use serde_json::Value;

mod composition;
mod composition_replay;
mod correction;
pub(crate) mod fixture;
mod jobs;
mod jobs_recovery;
mod preflight;
mod public_derivation;
mod publication;
mod refusals;
mod witness;

use fixture::Fixture;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn selected_capture_save_reopen_retains_all_three_verified_views_without_live_sources(
) -> TestResult {
    let fixture = Fixture::new().await?;
    let objects = temporary()?;
    let archive = temporary()?;
    let path = archive.path().join("selected.json");
    let dataset = fixture.capture(objects.path())?;
    let assessment = dataset
        .verified_participation()?
        .assessment(&fixture.performance.id)
        .ok_or("participation assessment")?;
    check!(eq; assessment.outcome(), ParticipationOutcome::Unresolved);
    check!(eq; assessment.reasons().iter().collect::<Vec<_>>(), vec![
        census_domain::participation_candidates::ParticipationReason::HighSchoolStatusMissing,
        census_domain::participation_candidates::ParticipationReason::SummerBoundaryUnresolved,
    ]);
    check!(
        matches!(dataset.verified_current_affiliations()?.resolution(&fixture.roster_athlete),
        CurrentAffiliationResolution::Resolved { school, evidence_refs }
            if school == &fixture.roster_school && !evidence_refs.is_empty())
    );
    check!(dataset
        .verified_public_identities()?
        .public_id(fixture.roster_athlete.as_str())
        .is_some());
    let expected = witness::capture(&dataset)?;
    let lineage = dataset.lineage.clone();
    let digest = dataset.archive_digest()?;
    let store_path = fixture.store.root().to_path_buf();
    let objects_path = objects.path().to_path_buf();
    dataset.save_frozen(&path)?;
    let wire: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    check!(wire.get("selected_proofs").is_some());
    check!(eq; wire.pointer("/selected_proofs/version").and_then(Value::as_u64), Some(2));
    check!(eq; fixture.store.store_format()?.lifecycle_revision, Some(7));
    check!(wire
        .get("data")
        .and_then(|value| value.get("selected_proofs"))
        .is_none());
    fixture.remove_sources()?;
    objects.close()?;
    drop(dataset);
    check!(!store_path.exists());
    check!(!objects_path.exists());
    let reopened = ExportDataset::reopen_frozen_with_digest(&path, &digest)?;
    check!(eq; reopened.lineage, lineage);
    check!(eq; reopened.archive_digest()?, digest);
    check!(eq; witness::capture(&reopened)?, expected);
    let copied = archive.path().join("recopied.json");
    reopened.save_frozen(&copied)?;
    drop(reopened);
    std::fs::remove_file(&path)?;
    std::fs::remove_dir_all(super::super::frozen::proof_directory(&path)?)?;
    let recopied = ExportDataset::reopen_frozen_with_digest(&copied, &digest)?;
    check!(eq; witness::capture(&recopied)?, expected);
    Ok(())
}

#[test]
fn legacy_frozen_archives_keep_the_original_envelope_and_unbound_reopen_behavior() -> TestResult {
    let directory = temporary()?;
    let store = Store::open(directory.path().join("store"))?;
    store.store_identity()?;
    let mut dataset = ExportDataset::load(&store)?;
    for revision in 19..=22 {
        dataset.bind_historical_policy_for_test(revision)?;
        let path = directory.path().join(format!("legacy-{revision}.json"));
        dataset.save_frozen(&path)?;
        let wire: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        check!(wire.get("selected_proofs").is_none());
        check!(wire
            .get("data")
            .and_then(|data| data.get("selection"))
            .is_none());
        check!(!super::super::frozen::proof_directory(&path)?.exists());
        let unbound = ExportDataset::reopen_frozen(&path)?;
        let bound = ExportDataset::reopen_frozen_with_digest(&path, &dataset.archive_digest()?)?;
        check!(eq; unbound.lineage, dataset.lineage);
        check!(eq; bound.lineage, dataset.lineage);
        check!(eq; serde_json::to_value(&unbound)?, serde_json::to_value(&dataset)?);
        check!(unbound.verified_participation().is_err());
        check!(unbound.verified_current_affiliations().is_err());
        check!(unbound.verified_public_identities().is_err());
    }
    Ok(())
}

fn temporary() -> TestResult<tempfile::TempDir> {
    Ok(tempfile::tempdir_in(std::fs::canonicalize(
        std::env::temp_dir(),
    )?)?)
}
