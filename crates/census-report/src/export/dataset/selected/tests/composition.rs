use super::{witness, ExportDataset, Fixture, ParticipationOutcome, Table, TestResult, Value};
use crate::export::{CurrentAffiliationResolution, ParticipationDiagnostic};
use census_domain::model::{
    serialized_digest, CanonicalAthlete, CanonicalEvent, CanonicalPerformance, CanonicalTeam,
    Evidence, GradYear, Mark, PublishedGraduation, SourceRef,
};
use serde_json::json;

#[tokio::test]
async fn same_athlete_roster_and_two_meets_retain_missing_window_exclusions_after_source_removal(
) -> TestResult {
    for two_meets in [false, true] {
        let fixture = Fixture::composed(two_meets).await?;
        let objects = super::temporary()?;
        let archive = super::temporary()?;
        let dataset = fixture.capture(objects.path())?;
        check!(eq; fixture.roster_athlete, fixture.performance.athlete);
        check!(eq; dataset.performances.len(), if two_meets { 2 } else { 1 });
        for performance in &dataset.performances {
            let row = dataset
                .verified_participation()?
                .assessment(&performance.id)
                .ok_or("participation")?;
            check!(eq; row.outcome(), ParticipationOutcome::Unresolved);
            check!(row.diagnostics().is_empty());
            check!(eq; row.reasons().iter().collect::<Vec<_>>(), vec![census_domain::participation_candidates::ParticipationReason::HighSchoolStatusMissing, census_domain::participation_candidates::ParticipationReason::SummerBoundaryUnresolved]);
            check!(dataset.performances.contains(performance));
        }
        check!(
            matches!(dataset.verified_current_affiliations()?.resolution(&fixture.roster_athlete), CurrentAffiliationResolution::Resolved { school, .. } if school == &fixture.roster_school)
        );
        let bests = crate::bests::build_with_exclusions(
            &dataset,
            &crate::bests::Options {
                scope: crate::report::Scope::AllSources,
                ..crate::bests::Options::default()
            },
        )?;
        check!(bests.selections.is_empty());
        check!(eq; bests.participation_exclusions.len(), dataset.performances.len());
        for performance in &dataset.performances {
            let exclusion = bests
                .participation_exclusions
                .iter()
                .find(|row| row.performance_id() == &performance.id)
                .ok_or("retained unresolved result")?;
            check!(eq; exclusion.outcome(), crate::bests::ParticipationExclusionOutcome::Unresolved);
        }
        let expected = witness::capture(&dataset)?;
        let path = archive.path().join("composed.json");
        let digest = dataset.archive_digest()?;
        dataset.save_frozen(&path)?;
        fixture.remove_sources()?;
        objects.close()?;
        drop(dataset);
        let replayed = ExportDataset::reopen_frozen_with_digest(&path, &digest)?;
        check!(eq; witness::capture(&replayed)?, expected);
    }
    Ok(())
}

#[tokio::test]
async fn roster_context_preserves_unknown_and_conflicting_cohort_evidence() -> TestResult {
    for conflict in [false, true] {
        let fixture = Fixture::composed(false).await?;
        let mut athlete = athlete(&fixture)?;
        if conflict {
            athlete.published_graduations.push(PublishedGraduation {
                grad_year: GradYear::new(2028).ok_or("conflicting year")?,
                source: roster_evidence(&fixture)?.source,
            });
        } else {
            athlete.evidence.push(Evidence::derived(
                SourceRef::id("unknown-status"),
                "2026-10-01",
                "unverified eligibility assertion",
            ));
        }
        fixture.store.append(Table::Athletes, &athlete)?;
        let objects = super::temporary()?;
        let dataset = fixture.capture(objects.path())?;
        let assessment = dataset
            .verified_participation()?
            .assessment(&fixture.performance.id)
            .ok_or("participation")?;
        check!(eq; assessment.outcome(), ParticipationOutcome::Unresolved);
        check!(assessment.diagnostics().contains(&if conflict {
            ParticipationDiagnostic::CohortUnresolved
        } else {
            ParticipationDiagnostic::UnverifiedAdditionalEvidence
        }));
        let expected = witness::capture(&dataset)?;
        let archive = super::temporary()?;
        let path = archive.path().join("unresolved.json");
        let digest = dataset.archive_digest()?;
        dataset.save_frozen(&path)?;
        fixture.remove_sources()?;
        objects.close()?;
        drop(dataset);
        check!(eq; witness::capture(&ExportDataset::reopen_frozen_with_digest(&path, &digest)?)?, expected);
    }
    Ok(())
}

#[tokio::test]
async fn forged_roster_owner_row_and_source_never_create_context_witnesses() -> TestResult {
    for field in ["provider_athlete_id", "row_sha256", "source"] {
        let fixture = Fixture::composed(false).await?;
        let mut athlete = athlete(&fixture)?;
        let evidence = athlete
            .evidence
            .iter_mut()
            .find(|evidence| is_roster(evidence))
            .ok_or("roster proof")?;
        if field == "source" {
            evidence.source.id = "milesplit_wi".into();
        } else {
            let mut note: Value =
                serde_json::from_str(evidence.note.as_deref().ok_or("row metadata")?)?;
            *note.get_mut(field).ok_or("note field")? = if field == "row_sha256" {
                json!("a".repeat(64))
            } else {
                json!("14222593")
            };
            evidence.note = Some(note.to_string());
        }
        fixture.store.append(Table::Athletes, &athlete)?;
        let objects = super::temporary()?;
        check!(fixture.capture(objects.path()).is_err());
    }
    Ok(())
}

#[tokio::test]
async fn genuine_roster_for_foreign_person_and_wrong_entity_kind_cannot_be_laundered() -> TestResult
{
    let fixture = Fixture::new().await?;
    let mut person = fixture
        .store
        .snapshot()
        .athletes()?
        .into_iter()
        .find(|row| row.id == fixture.performance.athlete)
        .ok_or("result subject")?;
    person.evidence.push(roster_evidence(&fixture)?);
    fixture.store.append(Table::Athletes, &person)?;
    let objects = super::temporary()?;
    check!(fixture.capture(objects.path()).is_err());
    let fixture = Fixture::composed(false).await?;
    let mut team = fixture
        .store
        .scan::<CanonicalTeam>(Table::Teams)?
        .into_iter()
        .find(|team| team.id == fixture.performance.team)
        .ok_or("team")?;
    let evidence = roster_evidence(&fixture)?;
    team.evidence.push(evidence.clone());
    fixture.store.append(Table::Teams, &team)?;
    let objects = super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    unresolved(
        &dataset,
        &fixture.performance,
        ParticipationDiagnostic::UnverifiedAdditionalEvidence,
    )?;
    check!(dataset
        .verified_participation()?
        .assessment(&fixture.performance.id)
        .ok_or("participation")?
        .evidence_refs()
        .contains(&format!(
            "canonical_evidence:{}",
            serialized_digest(&evidence)?
        )));
    Ok(())
}

#[tokio::test]
async fn verified_second_meet_context_cannot_authenticate_a_foreign_event_or_conflicting_mark(
) -> TestResult {
    for foreign_event in [true, false] {
        let fixture = Fixture::composed(true).await?;
        if foreign_event {
            let second = fixture
                .store
                .scan::<CanonicalPerformance>(Table::Performances)?
                .into_iter()
                .find(|row| row.source_key == "milesplit_result:201782264")
                .ok_or("second result")?;
            let mut event = fixture
                .store
                .scan::<CanonicalEvent>(Table::Events)?
                .into_iter()
                .find(|event| event.id == fixture.performance.event)
                .ok_or("first event")?;
            event.evidence.extend(second.evidence);
            fixture.store.append(Table::Events, &event)?;
        } else {
            let mut changed = fixture.performance.clone();
            changed.mark = Mark::Raw("DQ".into());
            fixture.store.append(Table::Performances, &changed)?;
        }
        let objects = super::temporary()?;
        let dataset = fixture.capture(objects.path())?;
        let assessment = dataset
            .verified_participation()?
            .assessment(&fixture.performance.id)
            .ok_or("participation")?;
        check!(eq; assessment.outcome(), ParticipationOutcome::Unresolved);
        check!(assessment.diagnostics().contains(&if foreign_event {
            ParticipationDiagnostic::UnverifiedAdditionalEvidence
        } else {
            ParticipationDiagnostic::CanonicalConflict
        }));
    }
    Ok(())
}

#[tokio::test]
async fn unavailable_roster_and_tampered_frozen_roster_cannot_become_verified_context() -> TestResult
{
    let fixture = Fixture::composed(false).await?;
    let evidence = roster_evidence(&fixture)?;
    let note: Value = serde_json::from_str(evidence.note.as_deref().ok_or("roster note")?)?;
    let body_digest = note
        .get("capture_sha256")
        .and_then(Value::as_str)
        .ok_or("body digest")?;
    let body_path = fixture
        .store
        .http_cache_dir()
        .join("archive/bodies")
        .join(format!("{body_digest}.body"));
    let original = std::fs::read(&body_path)?;
    std::fs::remove_file(&body_path)?;
    let objects = super::temporary()?;
    let missing = fixture.capture(objects.path())?;
    unresolved(
        &missing,
        &fixture.performance,
        ParticipationDiagnostic::UnverifiedAdditionalEvidence,
    )?;
    check!(eq; missing.verified_current_affiliations()?.resolution(&fixture.roster_athlete), &CurrentAffiliationResolution::Unknown);
    std::fs::write(&body_path, &original)?;
    let objects = super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    let archive = super::temporary()?;
    let path = archive.path().join("tampered.json");
    let digest = dataset.archive_digest()?;
    dataset.save_frozen(&path)?;
    let proof_path = super::super::super::frozen::proof_directory(&path)?.join(body_digest);
    check!(eq; std::fs::read(&proof_path)?, original);
    std::fs::write(&proof_path, b"forged roster body")?;
    fixture.remove_sources()?;
    objects.close()?;
    drop(dataset);
    check!(ExportDataset::reopen_frozen_with_digest(&path, &digest).is_err());
    Ok(())
}

fn athlete(fixture: &Fixture) -> TestResult<CanonicalAthlete> {
    Ok(fixture
        .store
        .snapshot()
        .athletes()?
        .into_iter()
        .find(|athlete| athlete.id == fixture.roster_athlete)
        .ok_or("roster subject")?)
}

fn roster_evidence(fixture: &Fixture) -> TestResult<Evidence> {
    Ok(athlete(fixture)?
        .evidence
        .into_iter()
        .find(is_roster)
        .ok_or("roster evidence")?)
}

fn is_roster(evidence: &Evidence) -> bool {
    evidence
        .note
        .as_deref()
        .is_some_and(|note| note.contains("roster_positive_row_metadata_v1"))
}

fn unresolved(
    dataset: &ExportDataset,
    performance: &CanonicalPerformance,
    diagnostic: ParticipationDiagnostic,
) -> TestResult {
    let assessment = dataset
        .verified_participation()?
        .assessment(&performance.id)
        .ok_or("participation")?;
    check!(eq; assessment.outcome(), ParticipationOutcome::Unresolved);
    check!(assessment.diagnostics().contains(&diagnostic));
    Ok(())
}
