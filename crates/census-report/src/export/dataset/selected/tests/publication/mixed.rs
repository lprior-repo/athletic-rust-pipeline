use super::{publication, publish, ExportDataset, Fixture, GradYear, Scope, TestResult};
use crate::bests::{ParticipationExclusionOutcome, ParticipationExclusionReason, SelectionOutcome};
use crate::export::ParticipationOutcome;
use census_domain::model::CanonicalPerformance;
use census_domain::participation_candidates::ParticipationReason;

mod readback;

#[tokio::test]
async fn non_school_exclusions_and_missing_status_window_remain_distinct_after_offline_replay(
) -> TestResult {
    for (division, reason, reason_label) in [
        (
            "Collegiate",
            ParticipationReason::CollegiateParticipation,
            "collegiate_participation",
        ),
        (
            "Middle School",
            ParticipationReason::MiddleSchoolParticipation,
            "middle_school_participation",
        ),
        (
            "Post Graduation",
            ParticipationReason::PostGraduationParticipation,
            "post_graduation_participation",
        ),
    ] {
        let fixture = Fixture::mixed_two_meets(division).await?;
        let objects = super::super::temporary()?;
        let publication_root = super::super::temporary()?;
        let dataset = fixture.capture(objects.path())?;
        let unresolved = fixture.performance.clone();
        let excluded = dataset
            .performances
            .iter()
            .find(|row| row.source_key == "milesplit_result:201782264")
            .ok_or("stronger retained non-school result")?
            .clone();
        let expected_outcome = assert_outcome(&dataset, &unresolved, &excluded, reason)?;
        let public_id = dataset
            .verified_public_identities()?
            .public_id(unresolved.athlete.as_str())
            .ok_or("accepted public identity")?
            .as_str()
            .to_owned();
        let assessment = dataset
            .verified_participation()?
            .assessment(&excluded.id)
            .ok_or("retained excluded assessment")?;
        let expected = readback::Expected {
            public_id,
            unresolved,
            excluded,
            reason_label,
            unresolved_evidence_refs: dataset
                .verified_participation()?
                .assessment(&fixture.performance.id)
                .ok_or("unresolved assessment")?
                .evidence_refs()
                .to_vec(),
            evidence_refs: assessment.evidence_refs().to_vec(),
            input_generation: dataset.lineage.input_generation.clone(),
        };
        let witness = super::super::witness::capture(&dataset)?;
        let lineage = dataset.lineage.clone();
        let path = publish(
            &fixture,
            &dataset,
            &publication_root.path().join("publication"),
        )?;
        let generation = path.parent().ok_or("publication generation")?;
        let verified = publication::verify_published(&path)?;
        check!(eq; verified.workbook.mapped_athletes, 1);
        readback::artifacts(generation, &path, &expected)?;
        let store_path = fixture.store.root().to_path_buf();
        let cache_path = fixture.store.http_cache_dir();
        let staging_path = objects.path().to_path_buf();
        fixture.remove_sources()?;
        objects.close()?;
        drop(dataset);
        check!(!store_path.exists());
        check!(!cache_path.exists());
        check!(!staging_path.exists());
        let reopened =
            publication::open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027)?;
        check!(eq; super::super::witness::capture(&reopened.dataset)?, witness);
        check!(eq; assert_outcome(&reopened.dataset, &expected.unresolved, &expected.excluded, reason)?, expected_outcome);
        check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
        readback::artifacts(generation, &path, &expected)?;
    }
    Ok(())
}

fn assert_outcome(
    dataset: &ExportDataset,
    unresolved: &CanonicalPerformance,
    excluded: &CanonicalPerformance,
    reason: ParticipationReason,
) -> TestResult<SelectionOutcome> {
    check!(eq; dataset.performances.len(), 2);
    check!(ne; unresolved.meet, excluded.meet);
    check!(eq; unresolved.athlete, excluded.athlete);
    check!(eq; unresolved.source_key, "milesplit_result:201782263");
    check!(eq; crate::bests::mark_text(&unresolved.mark), "13-9");
    check!(eq; crate::bests::mark_text(&excluded.mark), "14-9");
    let participation = dataset.verified_participation()?;
    let accepted = participation
        .assessment(&unresolved.id)
        .ok_or("unresolved assessment")?;
    check!(eq; accepted.outcome(), ParticipationOutcome::Unresolved);
    check!(eq; accepted.reasons().iter().collect::<Vec<_>>(), vec![ParticipationReason::HighSchoolStatusMissing, ParticipationReason::SummerBoundaryUnresolved]);
    check!(accepted.diagnostics().is_empty());
    let rejected = participation
        .assessment(&excluded.id)
        .ok_or("excluded assessment")?;
    check!(eq; rejected.outcome(), ParticipationOutcome::Excluded);
    check!(eq; rejected.reasons().iter().collect::<Vec<_>>(), vec![ParticipationReason::SummerBoundaryUnresolved, reason]);
    check!(rejected.diagnostics().is_empty());
    for assessment in [accepted, rejected] {
        for prefix in ["acquisition/", "metadata/"] {
            check!(assessment
                .evidence_refs()
                .iter()
                .any(|value| value.starts_with(prefix)));
        }
    }
    let outcome = crate::bests::build_with_exclusions(
        dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: vec![
                census_domain::UsJurisdiction::Alabama,
                census_domain::UsJurisdiction::Wisconsin,
            ],
        },
    )?;
    check!(outcome.selections.is_empty());
    check!(outcome.combined_exclusions.is_empty());
    check!(eq; outcome.participation_exclusions.len(), 2);
    let rejection = outcome
        .participation_exclusions
        .iter()
        .find(|row| row.performance_id() == &excluded.id)
        .ok_or("exact exclusion")?;
    check!(eq; rejection.performance_id(), &excluded.id);
    check!(eq; rejection.event_id(), &excluded.event);
    check!(eq; rejection.athlete_id(), &excluded.athlete);
    check!(eq; rejection.outcome(), ParticipationExclusionOutcome::Excluded);
    check!(eq; rejection.reason(), ParticipationExclusionReason::Assessment(rejected.reasons()));
    let missing = outcome
        .participation_exclusions
        .iter()
        .find(|row| row.performance_id() == &unresolved.id)
        .ok_or("missing-status/window exclusion")?;
    check!(eq; missing.outcome(), ParticipationExclusionOutcome::Unresolved);
    check!(eq; missing.reason(), ParticipationExclusionReason::Assessment(accepted.reasons()));
    Ok(outcome)
}
