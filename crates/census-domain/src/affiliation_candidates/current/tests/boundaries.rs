use super::*;

#[test]
fn exact_day_affiliation_is_not_extended_to_neighboring_days() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let effective = day("2026-10-10")?;
    let mut fact = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    fact.temporal = TemporalScope::ExactDay(&effective);
    let assertions = [LocallyCheckedAssertion::check(fact)?];
    for (date, expected) in [
        ("2026-10-09", CurrentAffiliationOutcome::Unknown),
        (
            "2026-10-10",
            CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school),
        ),
        ("2026-10-11", CurrentAffiliationOutcome::Unknown),
    ] {
        let profile = profile(date)?;
        let request = request(&fixture, &profile)?;
        let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
        check!(eq; assessed.outcome, expected);
    }
    Ok(())
}

#[test]
fn explicit_departure_keeps_last_day_and_reports_ended_without_replacement() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let last_day = day("2026-09-30")?;
    let assertions = [LocallyCheckedAssertion::check(fixture.assertion(
        &evidence,
        &published,
        key("claim-A")?,
    )?)?];
    let changes = [fixture.change(
        &evidence,
        key("claim-A")?,
        ChangeKind::EndsRelationship {
            last_day: &last_day,
        },
    )?];
    for (date, expected) in [
        (
            "2026-09-30",
            CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school),
        ),
        (
            "2026-10-01",
            CurrentAffiliationOutcome::EndedWithoutReplacement,
        ),
    ] {
        let profile = profile(date)?;
        let request = request(&fixture, &profile)?;
        let assessed =
            assess_current_affiliation(request, inventory(request, &assertions, &changes))?;
        check!(eq; assessed.outcome, expected);
        check!(eq; assessed.facts.len(), 2);
    }
    Ok(())
}

#[test]
fn closed_interval_expiry_is_distinct_from_missing_evidence() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let interval = ProfileDateRangeV1::new(day("2026-09-01")?, day("2026-09-30")?)?;
    let mut fact = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    fact.temporal = TemporalScope::Closed(&interval);
    let assertions = [LocallyCheckedAssertion::check(fact)?];
    let profile = profile("2026-10-01")?;
    let request = request(&fixture, &profile)?;
    let ended = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; ended.outcome, CurrentAffiliationOutcome::EndedWithoutReplacement);
    check!(matches!(
        ended.facts.first(),
        Some(FactDisposition::Assertion {
            disposition: Disposition::ExplicitIntervalEnded,
            ..
        })
    ));
    let absent = assess_current_affiliation(request, inventory(request, &[], &[]))?;
    check!(eq; absent.outcome, CurrentAffiliationOutcome::Unknown);
    Ok(())
}

#[test]
fn an_unresolved_temporal_claim_prevents_a_resolved_school_candidate() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let a = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    let mut b = fixture.assertion(&evidence, &published, key("claim-B")?)?;
    b.temporal = TemporalScope::Unresolved(TemporalGap::Undated);
    let assertions = [
        LocallyCheckedAssertion::check(a)?,
        LocallyCheckedAssertion::check(b)?,
    ];
    let profile = profile("2026-10-10")?;
    let request = request(&fixture, &profile)?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::Unknown);
    check!(assessed.facts.iter().any(|fact| matches!(
        fact,
        FactDisposition::Assertion {
            disposition: Disposition::TemporalUnresolved(TemporalGap::Undated),
            ..
        }
    )));
    Ok(())
}
