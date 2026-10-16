use super::*;

#[test]
fn v4_in_season_support_cannot_substitute_division_for_status_or_erase_negative_status(
) -> TestResult {
    let fixture = fixture()?;
    let interval = range("2026-07-31", "2026-07-31")?;
    let supported = [fact(&fixture, &interval)?];
    let mut case = fixture.case()?;
    case.windows.clear();
    case.statuses.first_mut().ok_or("status")?.basis = StatusBasis::ExplicitScholasticResult;
    let observation = case.observation(&fixture);
    let result = assess_participation_with_in_season(
        fixture.query()?,
        &observation,
        InSeasonEvidence::Uncontested(&supported),
    )?;
    check!(eq; result.outcome, CandidateOutcome::Unresolved);
    check!(result.reasons.contains(Reason::HighSchoolStatusMissing));
    case.statuses.first_mut().ok_or("status")?.status = StudentStatus::Collegiate;
    let observation = case.observation(&fixture);
    let result = assess_participation_with_in_season(
        fixture.query()?,
        &observation,
        InSeasonEvidence::Uncontested(&supported),
    )?;
    check!(eq; result.outcome, CandidateOutcome::Excluded);
    check!(result.reasons.contains(Reason::CollegiateParticipation));
    Ok(())
}

#[test]
fn v4_season_support_requires_matching_date_cohort_and_exact_policy() -> TestResult {
    let fixture = fixture()?;
    let interval = range("2026-07-31", "2026-07-31")?;
    let other_day = range("2026-07-30", "2026-07-30")?;
    let old_policy = participation_policy_v3()?;
    let original = fact(&fixture, &interval)?;
    let mut wrong_cohort = original;
    wrong_cohort.identity.cohort = GradYear::new(2028).ok_or("cohort")?;
    let mut wrong_policy = original;
    wrong_policy.identity.participation_rule = &old_policy;
    let mut wrong_day = original;
    wrong_day.interval = &other_day;
    let mut case = fixture.case()?;
    case.windows.clear();
    let observation = case.observation(&fixture);
    for wrong in [wrong_cohort, wrong_policy, wrong_day] {
        let facts = [wrong];
        let result = assess_participation_with_in_season(
            fixture.query()?,
            &observation,
            InSeasonEvidence::Uncontested(&facts),
        )?;
        check!(eq; result.outcome, CandidateOutcome::Unresolved);
        check!(eq; result.reasons.iter().collect::<Vec<_>>(), vec![Reason::SummerBoundaryUnresolved]);
    }
    Ok(())
}

#[test]
fn season_conflict_is_invariant_under_duplicate_facts_and_input_order() -> TestResult {
    let fixture = fixture()?;
    let interval = range("2026-07-31", "2026-07-31")?;
    let first = fact(&fixture, &interval)?;
    let mut second = first;
    second.boundary_proof = fixture.proof("second-positive")?;
    let mut negative = first;
    negative.boundary_proof = fixture.proof("explicit-season-denial")?;
    let mut case = fixture.case()?;
    case.windows.clear();
    let observation = case.observation(&fixture);
    for supported in [vec![first, second], vec![second, first, first]] {
        for contrary in [vec![negative], vec![negative, negative]] {
            let result = assess_participation_with_in_season(
                fixture.query()?,
                &observation,
                InSeasonEvidence::Conflicted {
                    supported: &supported,
                    contrary: &contrary,
                },
            )?;
            check!(eq; result.outcome, CandidateOutcome::Unresolved);
            check!(result.reasons.contains(Reason::ParticipationWindowConflict));
        }
    }
    Ok(())
}
