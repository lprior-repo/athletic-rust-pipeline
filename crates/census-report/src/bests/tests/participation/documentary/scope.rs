use super::*;

#[test]
fn documentary_exclusions_refuse_every_partial_selected_scope() -> TestResult {
    let fixture = crate::export::selected_test_fixture::consumer::source_births()?;
    let (dataset, _objects) =
        crate::export::selected_test_fixture::consumer::capture(&fixture.store)?;
    let (_, observation, _) = observations()?;
    let row = DocumentaryParticipationRow::fixture(
        observation.performance.id.clone(),
        observation.event.id.clone(),
        observation.performance.athlete.clone(),
        CandidateOutcome::EligibleCandidate,
        ParticipationReasons::default(),
        Vec::new(),
    );
    let view = view(&dataset, row)?;
    let complete = options(&dataset)?;
    check!(eq; reduce(&dataset, &view, &complete)?.participation_exclusions.len(), 1);
    for cohort in [None, Some(2026), Some(2028)] {
        let mut options = options(&dataset)?;
        options.grad_year = cohort;
        check!(reduce(&dataset, &view, &options).is_err());
    }
    for limit in [0, 1, usize::MAX] {
        let mut options = options(&dataset)?;
        options.limit = Some(limit);
        check!(reduce(&dataset, &view, &options).is_err());
    }
    let mut reversed = complete.jurisdictions.clone();
    reversed.reverse();
    check!(reversed != complete.jurisdictions);
    let mut extra = complete.jurisdictions.clone();
    extra.push(UsJurisdiction::Michigan);
    for jurisdictions in [Vec::new(), vec![UsJurisdiction::Alabama], reversed, extra] {
        let mut options = options(&dataset)?;
        options.jurisdictions = jurisdictions;
        check!(reduce(&dataset, &view, &options).is_err());
    }
    let mut scoped = options(&dataset)?;
    scoped.scope = Scope::Core;
    check!(reduce(&dataset, &view, &scoped).is_err());
    check!(eq; reduce(&dataset, &view, &complete)?.participation_exclusions.len(), 1);
    Ok(())
}
