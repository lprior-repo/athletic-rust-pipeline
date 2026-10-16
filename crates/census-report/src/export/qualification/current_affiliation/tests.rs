use super::*;
use crate::export::ExportDataset;
use census_domain::model::*;
use census_store::{Store, Table};

pub(super) mod fixture;
pub(super) mod identity;
mod performance;
mod refusals;
mod retention;
mod temporal;
mod withholding;

use fixture::Fixture;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn current_roster_requires_original_archived_metadata_and_exact_source_row() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let before = serde_json::to_vec(&dataset)?;
    let pinned = PinnedCurrentAffiliations::read(&dataset, &snapshot)?;
    check!(pinned.unavailable().is_empty());
    let verified = pinned.into_verified();
    check!(eq; verified.input_generation(), dataset.lineage.input_generation.as_str());
    check!(eq; verified.profile_id(), &fixture.profile.id()?);
    check!(eq; verified.as_of(), &fixture.profile.input().as_of);
    check!(eq; verified.resolution(&fixture.athlete).school(), Some(&fixture.school));
    let audit = verified
        .audit(&fixture.athlete)
        .ok_or("affiliation audit")?;
    check!(audit.unavailable().is_empty());
    check!(eq; audit.references().len(), 2);
    let refs = verified.resolution(&fixture.athlete).evidence_refs();
    check!(eq; refs, audit.references());
    check!(eq; refs.len(), 2);
    check!(refs
        .iter()
        .any(|value| value.contains("#capture=") && value.contains("&offset=")));
    check!(refs
        .iter()
        .any(|value| value.starts_with("archive/captures/") && value.ends_with(".meta.json")));
    let repeated = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    check!(eq; repeated.resolution(&fixture.athlete), verified.resolution(&fixture.athlete));
    check!(eq; serde_json::to_vec(&dataset)?, before);
    Ok(())
}

#[test]
fn old_or_future_roster_observation_never_becomes_continuing_tenure() -> TestResult {
    for cutoff in ["2026-10-06", "2026-10-08"] {
        let fixture = Fixture::new("2026-10-07T12:00:00Z", cutoff, true)?;
        let snapshot = fixture.store.snapshot();
        let dataset = fixture.dataset(&snapshot)?;
        let pinned = PinnedCurrentAffiliations::read(&dataset, &snapshot)?;
        check!(pinned
            .unavailable()
            .get(&fixture.athlete)
            .is_some_and(|gaps| gaps.contains(&UnavailableAffiliation::OutsideCaptureDay)));
        check!(eq; pinned.into_verified().resolution(&fixture.athlete), &CurrentAffiliationResolution::Unknown);
    }
    Ok(())
}

#[test]
fn accepted_alias_transfer_selects_current_school_without_mutating_history() -> TestResult {
    let fixture = Fixture::new("2026-10-06T12:00:00Z", "2026-10-07", true)?;
    let historical = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .find(|row| row.id == fixture.athlete)
        .ok_or("historical athlete")?;
    performance::append_performance(&fixture.store, &historical)?;
    let retained_performances = serde_json::to_vec(
        &fixture
            .store
            .scan::<CanonicalPerformance>(Table::Performances)?,
    )?;
    let (other, current_school, _, _) = fixture::append_roster(
        &fixture.store,
        "26848",
        "Current High",
        "2026-10-07T12:00:00Z",
    )?;
    let canonical =
        identity::accept_pair(&fixture.store, &fixture.writer, &fixture.athlete, &other)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let before = serde_json::to_vec(&dataset)?;
    let resolved = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    check!(eq; resolved.rows.len(), 1);
    check!(eq; resolved.resolution(&canonical).school(), Some(&current_school));
    let audit = resolved.audit(&canonical).ok_or("transfer audit")?;
    check!(eq; audit.references().len(), 2);
    check!(eq; audit.unavailable(), &["outside_capture_day".to_string()]);
    check!(dataset
        .athletes
        .iter()
        .any(|row| row.school == fixture.school));
    check!(dataset
        .athletes
        .iter()
        .any(|row| row.school == current_school));
    let selected = crate::report::Derivation::selected(
        &dataset,
        crate::report::Scope::AllSources,
        None,
        &[census_domain::UsJurisdiction::Wisconsin],
        &resolved,
    )?;
    check!(eq; selected.athletes().len(), 1);
    let row = selected
        .athletes()
        .first()
        .ok_or("selected current school athlete")?;
    check!(eq; &row.id, &canonical);
    check!(eq; &row.school, &current_school);
    let retained = dataset
        .performances
        .first()
        .ok_or("retained historical performance")?;
    check!(eq; dataset.teams.get(&retained.team).ok_or("historical team")?.school, fixture.school);
    check!(eq; serde_json::to_vec(&fixture.store.scan::<CanonicalPerformance>(Table::Performances)?)?, retained_performances);
    withholding::withdraw(&fixture, &other)?;
    let after_snapshot = fixture.store.snapshot();
    let after_dataset = fixture.dataset(&after_snapshot)?;
    let after = PinnedCurrentAffiliations::read(&after_dataset, &after_snapshot)?.into_verified();
    check!(eq; after.resolution(&canonical), &CurrentAffiliationResolution::Unknown);
    identity::assert_unselected(&after_dataset, &after, &canonical)?;
    check!(eq; serde_json::to_vec(&fixture.store.scan::<CanonicalPerformance>(Table::Performances)?)?, retained_performances);
    check!(eq; PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified().resolution(&canonical).school(), Some(&current_school));
    check!(eq; serde_json::to_vec(&dataset)?, before);
    Ok(())
}

#[test]
fn accepted_same_day_conflicting_rosters_remain_conflicted() -> TestResult {
    let fixture = Fixture::new("2026-10-07T09:00:00Z", "2026-10-07", true)?;
    let (other, _, _, _) = fixture::append_roster(
        &fixture.store,
        "26848",
        "Current High",
        "2026-10-07T12:00:00Z",
    )?;
    let canonical =
        identity::accept_pair(&fixture.store, &fixture.writer, &fixture.athlete, &other)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let resolved = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    check!(eq; resolved.rows.len(), 1);
    check!(eq; resolved.resolution(&canonical), &CurrentAffiliationResolution::Conflicted);
    let audit = resolved.audit(&canonical).ok_or("conflict audit")?;
    check!(eq; audit.references().len(), 4);
    check!(eq; audit.references().iter().filter(|reference| reference.contains("#capture=")).count(), 2);
    check!(eq; audit.references().iter().filter(|reference| reference.starts_with("archive/captures/")).count(), 2);
    check!(audit.unavailable().is_empty());
    identity::assert_unselected(&dataset, &resolved, &canonical)?;
    Ok(())
}

#[test]
fn unaccepted_identity_cannot_gain_authority_from_roster_membership() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", false)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let pinned = PinnedCurrentAffiliations::read(&dataset, &snapshot)?;
    check!(pinned
        .unavailable()
        .get(&fixture.athlete)
        .is_some_and(|gaps| gaps.contains(&UnavailableAffiliation::IdentityUnaccepted)));
    check!(eq; pinned.into_verified().resolution(&fixture.athlete), &CurrentAffiliationResolution::Unknown);
    Ok(())
}
