use super::*;
use census_domain::model::Id;
use census_domain::participation_candidates::{CandidateOutcome, ParticipationReasons};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn documentary_collision_with_structured_row_refuses_even_outside_positive_scope(
) -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let performance = dataset
        .performances
        .first()
        .ok_or("structured observation")?;
    let options = crate::bests::Options {
        scope: crate::report::Scope::AllSources,
        grad_year: Some(1901),
        limit: None,
        jurisdictions: Vec::new(),
    };
    let empty = crate::report::Derivation::with_participation_reducer_fixture(
        &dataset,
        &options,
        |derivation| Ok(derivation.performances().is_empty()),
    )?;
    check!(empty);
    let before = dataset.performances.clone();
    let mut budget = Admission::new("documentary collision test");
    let error = guards::collision(&dataset, &BTreeMap::new(), &performance.id, &mut budget)
        .err()
        .ok_or("structured collision refusal")?;
    check!(error
        .to_string()
        .contains("collides with structured source"));
    check!(eq; dataset.performances, before);
    Ok(())
}

#[test]
fn documentary_duplicate_and_conflicting_identity_never_overwrite_prior_observation() -> TestResult
{
    let fixture = crate::export::selected_test_fixture::consumer::source_births()?;
    let (dataset, _objects) =
        crate::export::selected_test_fixture::consumer::capture(&fixture.store)?;
    let id = Id::mint("perf", &["synthetic-documentary-collision"]);
    let row = DocumentaryParticipationRow::fixture(
        id.clone(),
        Id::mint("event", &["first"]),
        Id::mint("athlete", &["first"]),
        CandidateOutcome::EligibleCandidate,
        ParticipationReasons::default(),
        vec!["original-reference".into()],
    );
    let rows = BTreeMap::from([(id.clone(), row)]);
    let mut budget = Admission::new("documentary collision test");
    for (event, athlete) in [
        ("first", "first"),
        ("conflicting-event", "conflicting-athlete"),
    ] {
        let incoming = DocumentaryParticipationRow::fixture(
            id.clone(),
            Id::mint("event", &[event]),
            Id::mint("athlete", &[athlete]),
            CandidateOutcome::EligibleCandidate,
            ParticipationReasons::default(),
            vec!["incoming-reference".into()],
        );
        let error = guards::collision(&dataset, &rows, incoming.performance_id(), &mut budget)
            .err()
            .ok_or("documentary collision refusal")?;
        check!(error
            .to_string()
            .contains("duplicate or conflicting source observations"));
        check!(eq; rows.len(), 1);
        check!(eq; rows.get(&id).ok_or("original row")?.assessment().evidence_refs(), &["original-reference".to_string()]);
    }
    let different = Id::mint("perf", &["distinct-documentary-observation"]);
    guards::collision(&dataset, &rows, &different, &mut budget)?;
    Ok(())
}

#[test]
fn documentary_collision_comparisons_cannot_reset_the_cumulative_work_admission() -> TestResult {
    let fixture = crate::export::selected_test_fixture::consumer::source_births()?;
    let (dataset, _objects) =
        crate::export::selected_test_fixture::consumer::capture(&fixture.store)?;
    let id = Id::mint("perf", &["synthetic-documentary-prior"]);
    let row = DocumentaryParticipationRow::fixture(
        id.clone(),
        Id::mint("event", &["first"]),
        Id::mint("athlete", &["first"]),
        CandidateOutcome::EligibleCandidate,
        ParticipationReasons::default(),
        Vec::new(),
    );
    let rows = BTreeMap::from([(id, row)]);
    let mut budget = Admission::new("documentary exhausted test");
    budget.charge(0, crate::grouping::STAGE_WORK)?;
    let different = Id::mint("perf", &["synthetic-documentary-next"]);
    check!(matches!(
        guards::collision(&dataset, &rows, &different, &mut budget),
        Err(crate::report::ReportError::Grouping(
            crate::grouping::GroupingError::BudgetExceeded {
                resource: "work units",
                ..
            }
        ))
    ));
    check!(eq; rows.len(), 1);
    Ok(())
}
