use super::*;

#[test]
fn current_school_candidates_follow_transfer_interval_boundaries() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let first = ProfileDateRangeV1::new(day("2026-09-01")?, day("2026-09-30")?)?;
    let next = ProfileDateRangeV1::new(day("2026-10-01")?, day("2027-06-15")?)?;
    let mut a = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    a.temporal = TemporalScope::Closed(&first);
    let mut b = fixture.assertion(&evidence, &published, key("claim-B")?)?;
    b.temporal = TemporalScope::Closed(&next);
    b.owner.target = OwnerTarget::School(&fixture.other_school);
    b.relationship = RelationshipCandidate::EnrolledAtSchool(&fixture.other_school);
    let assertions = [
        LocallyCheckedAssertion::check(a)?,
        LocallyCheckedAssertion::check(b)?,
    ];
    for (effective, school) in [
        ("2026-09-01", &fixture.school),
        ("2026-09-30", &fixture.school),
        ("2026-10-01", &fixture.other_school),
        ("2026-10-10", &fixture.other_school),
    ] {
        let profile = profile(effective)?;
        let request = request(&fixture, &profile)?;
        let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
        check!(eq; assessed.outcome, CurrentAffiliationOutcome::ResolvedSchoolCandidate(school));
        check!(eq; assessed.facts.len(), 2);
        check!(eq; assessed.profile_id, profile.id()?);
    }
    check!(eq; assertions.first().ok_or("first assertion")?.raw().relationship,
        RelationshipCandidate::EnrolledAtSchool(&fixture.school));
    Ok(())
}

#[test]
fn concurrent_school_candidates_remain_conflicted_in_both_orders() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let a = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    let mut b = fixture.assertion(&evidence, &published, key("claim-B")?)?;
    b.owner.target = OwnerTarget::School(&fixture.other_school);
    b.relationship = RelationshipCandidate::EnrolledAtSchool(&fixture.other_school);
    let a = LocallyCheckedAssertion::check(a)?;
    let b = LocallyCheckedAssertion::check(b)?;
    let profile = profile("2026-10-10")?;
    let request = request(&fixture, &profile)?;
    for assertions in [[a, b], [b, a]] {
        let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
        check!(eq; assessed.outcome, CurrentAffiliationOutcome::Conflicted);
        check!(eq; fact_ids(&assessed), vec![key("claim-A")?, key("claim-B")?]);
    }
    Ok(())
}

#[test]
fn same_school_relationships_retain_all_candidate_evidence() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let a = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    let mut b = fixture.assertion(&evidence, &published, key("claim-B")?)?;
    b.relationship = RelationshipCandidate::RepresentsSchoolProgram {
        school: &fixture.school,
        program: &fixture.program,
    };
    let assertions = [
        LocallyCheckedAssertion::check(b)?,
        LocallyCheckedAssertion::check(a)?,
    ];
    let profile = profile("2026-10-10")?;
    let request = request(&fixture, &profile)?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school));
    check!(eq; fact_ids(&assessed), vec![key("claim-A")?, key("claim-B")?]);
    check!(assessed.facts.iter().all(|fact| matches!(
        fact,
        FactDisposition::Assertion {
            disposition: Disposition::Candidate,
            ..
        }
    )));
    Ok(())
}

#[test]
fn mapped_source_alias_order_does_not_select_a_different_school() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let alias = crate::model::SourceSubject::new(
        crate::model::SourceNamespace::MilesplitAthlete,
        "700099".to_owned(),
    )?;
    let a = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    let mut b = fixture.assertion(&evidence, &published, key("claim-B")?)?;
    b.person = PersonCandidate::Mapped {
        person: &alias,
        athlete: &fixture.athlete,
        decision: key("alias-binding-UNVERIFIED")?,
        known: known(1, 3)?,
    };
    let a = LocallyCheckedAssertion::check(a)?;
    let b = LocallyCheckedAssertion::check(b)?;
    let profile = profile("2026-10-10")?;
    let request = request(&fixture, &profile)?;
    for assertions in [[a, b], [b, a]] {
        let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
        check!(eq; assessed.outcome, CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school));
        check!(eq; fact_ids(&assessed), vec![key("claim-A")?, key("claim-B")?]);
    }
    Ok(())
}
