use super::*;

fn refreshed_profile() -> TestResult<QueryProfileV1> {
    let mut input = profile("2026-10-10")?.input().clone();
    input.acquisition_window = ProfileDateRangeV1::new(day("2026-09-01")?, day("2026-10-31")?)?;
    Ok(QueryProfileV1::new(input)?)
}

#[test]
fn knowledge_cutoff_is_independent_of_the_new_source_observation_window() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let assertions = [LocallyCheckedAssertion::check(fixture.assertion(
        &evidence,
        &published,
        key("claim-A")?,
    )?)?];
    let profile = refreshed_profile()?;
    let request = request(&fixture, &profile)?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.request.acquisition_cutoff, instant("2026-08-01T12:00:00Z")?);
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school));
    Ok(())
}

#[test]
fn an_older_withdrawal_still_blocks_the_retained_school_claim() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let assertions = [LocallyCheckedAssertion::check(fixture.assertion(
        &evidence,
        &published,
        key("claim-A")?,
    )?)?];
    let changes = [fixture.change(
        &evidence,
        key("claim-A")?,
        ChangeKind::WithdrawsClaim {
            scope: &fixture.interval,
        },
    )?];
    let profile = refreshed_profile()?;
    let request = request(&fixture, &profile)?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &changes))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::Unknown);
    check!(assessed.facts.iter().any(|fact| matches!(
        fact,
        FactDisposition::Assertion {
            disposition: Disposition::WithdrawnClaim,
            ..
        }
    )));
    check!(eq; assessed.facts.len(), 2);
    Ok(())
}

#[test]
fn an_older_school_claim_can_conflict_with_newly_observed_positive_evidence() -> TestResult {
    let fixture = Fixture::new()?;
    let old = fixture.evidence()?;
    let metadata = crate::model::SourceCapture::new(
        &fixture.owner,
        fixture.metadata.body_hash().to_owned(),
        fixture.metadata.byte_len(),
        "2026-09-01".to_owned(),
        Some("https://roster.example.test/requested".to_owned()),
    )?;
    let mut raw = *old.raw();
    raw.metadata = &metadata;
    raw.acquired_at = instant("2026-09-01T12:00:00Z")?;
    raw.occurrence = key("new-occurrence")?;
    raw.manifest_key = key("new-manifest")?;
    raw.interpretation_key = key("new-interpretation")?;
    let new = LocallyCheckedEvidence::check(raw)?;
    let published = [&fixture.owner];
    let a = fixture.assertion(&old, &published, key("claim-A")?)?;
    let mut b = fixture.assertion(&new, &published, key("claim-B")?)?;
    b.owner.target = OwnerTarget::School(&fixture.other_school);
    b.relationship = RelationshipCandidate::EnrolledAtSchool(&fixture.other_school);
    let assertions = [
        LocallyCheckedAssertion::check(a)?,
        LocallyCheckedAssertion::check(b)?,
    ];
    let profile = refreshed_profile()?;
    let mut request = request(&fixture, &profile)?;
    request.acquisition_cutoff = instant("2026-10-10T12:00:00Z")?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::Conflicted);
    check!(eq; assessed.facts.len(), 2);
    Ok(())
}

#[test]
fn an_older_different_athlete_claim_keeps_its_disposition_without_blocking_the_candidate(
) -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let a = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    let mut b = fixture.assertion(&evidence, &published, key("claim-B")?)?;
    b.person = PersonCandidate::Mapped {
        person: &fixture.person,
        athlete: &fixture.other_athlete,
        decision: key("other-binding-UNVERIFIED")?,
        known: known(1, 3)?,
    };
    let assertions = [
        LocallyCheckedAssertion::check(a)?,
        LocallyCheckedAssertion::check(b)?,
    ];
    let profile = refreshed_profile()?;
    let request = request(&fixture, &profile)?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school));
    check!(assessed.facts.iter().any(|fact| matches!(
        fact,
        FactDisposition::Assertion {
            disposition: Disposition::DifferentAthlete,
            ..
        }
    )));
    Ok(())
}

#[test]
fn an_older_not_visible_claim_remains_auditable_without_blocking_the_candidate() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let a = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    let mut b = fixture.assertion(&evidence, &published, key("claim-B")?)?;
    b.known = known(2, 3)?;
    b.owner.target = OwnerTarget::School(&fixture.other_school);
    b.relationship = RelationshipCandidate::EnrolledAtSchool(&fixture.other_school);
    let assertions = [
        LocallyCheckedAssertion::check(a)?,
        LocallyCheckedAssertion::check(b)?,
    ];
    let profile = refreshed_profile()?;
    let request = request(&fixture, &profile)?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school));
    check!(assessed.facts.iter().any(|fact| matches!(
        fact,
        FactDisposition::Assertion {
            disposition: Disposition::NotVisibleAtKnownRevision,
            ..
        }
    )));
    Ok(())
}
