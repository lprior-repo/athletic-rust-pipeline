use super::*;

#[test]
fn independent_home_school_and_club_candidates_are_explicit_non_school_outcomes() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let profile = profile("2026-10-10")?;
    let request = request(&fixture, &profile)?;
    for kind in [
        NonSchoolKind::Independent,
        NonSchoolKind::HomeSchool,
        NonSchoolKind::ClubOnly,
    ] {
        let mut raw = fixture.assertion(&evidence, &published, key("claim-A")?)?;
        raw.relationship = RelationshipCandidate::NonSchool(kind);
        let assertions = [LocallyCheckedAssertion::check(raw)?];
        let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
        check!(eq; assessed.outcome, CurrentAffiliationOutcome::NonSchool(kind));
    }
    Ok(())
}

#[test]
fn coop_program_without_a_person_campus_binding_remains_unresolved() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let mut raw = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    raw.owner.target = OwnerTarget::Program(&fixture.program);
    raw.relationship = RelationshipCandidate::RepresentsCoopProgram {
        program: &fixture.program,
        member: CoopMemberCandidate::Unknown,
    };
    let assertions = [LocallyCheckedAssertion::check(raw)?];
    let profile = profile("2026-10-10")?;
    let request = request(&fixture, &profile)?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::CoopCampusUnresolved);
    check!(matches!(
        assessed.facts.first(),
        Some(FactDisposition::Assertion {
            disposition: Disposition::CoopMemberUnknown,
            ..
        })
    ));
    Ok(())
}

#[test]
fn coop_campus_and_program_intervals_must_both_cover_the_effective_day() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let campus = ProfileDateRangeV1::new(day("2026-10-01")?, day("2026-10-10")?)?;
    let mut raw = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    raw.owner.target = OwnerTarget::Program(&fixture.program);
    raw.relationship = RelationshipCandidate::RepresentsCoopProgram {
        program: &fixture.program,
        member: CoopMemberCandidate::Claimed {
            school: &fixture.other_school,
            athlete_member_binding: key("campus-binding-UNVERIFIED")?,
            source_person: &fixture.person,
            campus_binding_interval: &campus,
            program_membership_interval: &fixture.interval,
            membership_statement: raw.membership_statement,
            known: known(1, 3)?,
        },
    };
    let assertions = [LocallyCheckedAssertion::check(raw)?];
    for (date, expected) in [
        (
            "2026-10-10",
            CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.other_school),
        ),
        (
            "2026-10-11",
            CurrentAffiliationOutcome::CoopCampusUnresolved,
        ),
    ] {
        let profile = profile(date)?;
        let request = request(&fixture, &profile)?;
        let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
        check!(eq; assessed.outcome, expected);
    }
    Ok(())
}

#[test]
fn an_unbound_person_is_not_promoted_by_a_school_candidate() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let mut raw = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    raw.person = PersonCandidate::Unbound {
        person: &fixture.person,
        gap: SubjectGap::UnacceptedBinding,
    };
    let assertions = [LocallyCheckedAssertion::check(raw)?];
    let profile = profile("2026-10-10")?;
    let request = request(&fixture, &profile)?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::SubjectUnresolved);
    check!(matches!(
        assessed.facts.first(),
        Some(FactDisposition::Assertion {
            disposition: Disposition::SubjectUnresolved(SubjectGap::UnacceptedBinding),
            ..
        })
    ));
    Ok(())
}

#[test]
fn foreign_owner_and_different_athlete_are_not_current_school_evidence() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let foreign = [&fixture.other_owner];
    let raw = fixture.assertion(&evidence, &foreign, key("claim-A")?)?;
    check!(matches!(
        LocallyCheckedAssertion::check(raw),
        Err(AffiliationCandidateError::ForeignOwner)
    ));
    let published = [&fixture.owner];
    let mut raw = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    raw.person = PersonCandidate::Mapped {
        person: &fixture.person,
        athlete: &fixture.other_athlete,
        decision: key("other-person-binding-UNVERIFIED")?,
        known: known(1, 3)?,
    };
    let assertions = [LocallyCheckedAssertion::check(raw)?];
    let profile = profile("2026-10-10")?;
    let request = request(&fixture, &profile)?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::Unknown);
    check!(matches!(
        assessed.facts.first(),
        Some(FactDisposition::Assertion {
            disposition: Disposition::DifferentAthlete,
            ..
        })
    ));
    Ok(())
}
