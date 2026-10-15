use super::*;
use census_domain::model::{ReviewCase, ATHLETE_IDENTITY_FAMILY};

#[test]
fn source_decision_matches_historical_inputs() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let athlete = subject("graph-source", "51001")?;
    store.append(Table::Athletes, &athlete)?;
    compare(&store, &writer, &athlete.id.cast())
}

#[test]
fn competing_provider_identity_outside_first_batch_refuses() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let mut athletes = Vec::new();
    for index in 0..257 {
        athletes.push(subject(
            &format!("graph-batch-{index}"),
            &format!("51{index:04}"),
        )?);
    }
    athletes.sort_by(|left, right| left.id.cmp(&right.id));
    let first = athletes.first().ok_or("first athlete")?.clone();
    let before = crate::public_identity::context::Inputs {
        athletes: athletes.clone(),
        observations: Vec::new(),
        cases: Vec::new(),
        verdicts: Vec::new(),
    };
    check!(crate::public_identity::authority::accept_inputs(
        &before,
        &first.id.cast(),
        "2026-10-15"
    )
    .is_ok());
    let last = athletes.last_mut().ok_or("last athlete")?;
    last.source = first.source.clone();
    store.append_many(Table::Athletes, &athletes)?;
    let inputs = crate::public_identity::context::Inputs::capture(&store.snapshot())?;
    check!(crate::public_identity::authority::accept_inputs(
        &inputs,
        &first.id.cast(),
        "2026-10-15"
    )
    .is_err());
    compare(&store, &writer, &first.id.cast())
}

#[test]
fn unresolved_review_conflict_matches_historical_refusal() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let athlete = subject("graph-review", "51002")?;
    let mut index = AthleteIdentityIndex::default();
    index.observe(&athlete)?;
    let members = vec![athlete.id.cast()];
    let evidence = index.case_evidence("graph-review", "unresolved", &members)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        athlete.id.as_str(),
        "graph-review",
        "unresolved",
        evidence,
    );
    case.member_ids = members;
    store.append(Table::Athletes, &athlete)?;
    store.replace_many(Table::ReviewCases, &[case])?;
    let inputs = crate::public_identity::context::Inputs::capture(&store.snapshot())?;
    check!(crate::public_identity::authority::accept_inputs(
        &inputs,
        &athlete.id.cast(),
        "2026-10-15"
    )
    .is_err());
    compare(&store, &writer, &athlete.id.cast())
}

#[test]
fn reviewed_historical_fixture_keeps_same_source_outcomes() -> TestResult {
    let fixture = crate::public_identity::context::reviewed_fixture::ready_for_merge()?;
    fixture
        .writer
        .enroll_graph_admission_v1(&fixture.store, fixture.store.snapshot().revision())?;
    let candidate = fixture
        .candidates
        .last()
        .ok_or("unrelated historical candidate")?;
    compare(&fixture.store, &fixture.writer, candidate)
}
