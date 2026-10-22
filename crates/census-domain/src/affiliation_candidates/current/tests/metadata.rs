use super::*;

#[test]
fn retrospective_candidate_uses_separate_effective_and_known_dates_without_changing_v1(
) -> TestResult {
    let fixture = Fixture::new()?;
    let original = fixture.evidence()?;
    let metadata = crate::model::SourceCapture::new(
        &fixture.owner,
        fixture.metadata.body_hash().to_owned(),
        fixture.metadata.byte_len(),
        "2026-10-12".to_owned(),
        Some("https://roster.example.test/requested".to_owned()),
    )?;
    let mut raw = *original.raw();
    raw.metadata = &metadata;
    raw.acquired_at = instant("2026-10-12T12:00:00Z")?;
    let evidence = LocallyCheckedEvidence::check(raw)?;
    let published = [&fixture.owner];
    let assertions = [LocallyCheckedAssertion::check(fixture.assertion(
        &evidence,
        &published,
        key("claim-A")?,
    )?)?];
    let profile = profile("2026-10-10")?;
    let mut request = request(&fixture, &profile)?;
    request.known.captured_at = instant("2026-10-20T12:00:00Z")?;
    request.acquisition_cutoff = instant("2026-10-20T12:00:00Z")?;
    let legacy = CandidateQuery {
        athlete: request.athlete,
        as_of: &profile.input().as_of,
        sport: request.sport,
        known: request.known,
        acquisition_cutoff: request.acquisition_cutoff,
        rule_claim: &profile.input().policies.affiliation,
        grant: request.grant,
    };
    check!(matches!(
        select_candidates(legacy, &assertions, &[]),
        Err(AffiliationCandidateError::CurrentCompilerEnvelopeViolated)
    ));
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school));
    check!(eq; assessed.request.profile.input().as_of, day("2026-10-10")?);
    check!(eq; assessed.request.known.captured_at, instant("2026-10-20T12:00:00Z")?);
    check!(eq; assessed.facts.len(), 1);
    Ok(())
}

#[test]
fn newer_revision_claim_remains_invisible_to_the_older_declared_view() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let mut fact = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    fact.known = known(2, 3)?;
    let assertions = [LocallyCheckedAssertion::check(fact)?];
    let profile = profile("2026-10-10")?;
    let mut request = request(&fixture, &profile)?;
    let earlier = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; earlier.outcome, CurrentAffiliationOutcome::Unknown);
    check!(matches!(
        earlier.facts.first(),
        Some(FactDisposition::Assertion {
            disposition: Disposition::NotVisibleAtKnownRevision,
            ..
        })
    ));
    request.known.revision = fixture::revision(2)?;
    let later = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; later.outcome, CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school));
    check!(eq; earlier.outcome, CurrentAffiliationOutcome::Unknown);
    Ok(())
}

#[test]
fn the_inventory_must_match_every_declared_view_coordinate() -> TestResult {
    let fixture = Fixture::new()?;
    let profile = profile("2026-10-10")?;
    let request = request(&fixture, &profile)?;
    let mut different = [request.known; 5];
    let [lineage, revision, captured, closure, family] = &mut different;
    lineage.lineage = key("other-lineage")?;
    revision.revision = fixture::revision(2)?;
    captured.captured_at = instant("2026-10-11T12:00:00Z")?;
    closure.closure_object = key("other-closure")?;
    family.family_inventory = key("other-family")?;
    for declared_view in different {
        let inventory = CurrentAffiliationInventory {
            declared_view,
            assertions: &[],
            changes: &[],
        };
        check!(matches!(
            assess_current_affiliation(request, inventory),
            Err(CurrentAffiliationError::InventoryViewMismatch)
        ));
    }
    Ok(())
}

#[test]
fn the_assessed_sport_must_be_selected_in_the_exact_profile() -> TestResult {
    let fixture = Fixture::new()?;
    let profile = profile("2026-10-10")?;
    let mut request = request(&fixture, &profile)?;
    for sport in [Sport::IndoorTrack, Sport::OutdoorTrack, Sport::Unknown] {
        request.sport = sport;
        check!(matches!(
            assess_current_affiliation(request, inventory(request, &[], &[])),
            Err(CurrentAffiliationError::SportNotSelected)
        ));
    }
    Ok(())
}

#[test]
fn acquisition_knowledge_cutoff_must_not_follow_the_frozen_instant() -> TestResult {
    let fixture = Fixture::new()?;
    let profile = profile("2026-10-10")?;
    let mut request = request(&fixture, &profile)?;
    request.acquisition_cutoff = instant("2026-10-10T12:00:01Z")?;
    check!(matches!(
        assess_current_affiliation(request, inventory(request, &[], &[])),
        Err(CurrentAffiliationError::AcquisitionCutoffAfterFrozenInstant)
    ));
    Ok(())
}

#[test]
fn retained_active_affiliation_is_not_expired_by_the_new_observation_window() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let assertions = [LocallyCheckedAssertion::check(fixture.assertion(
        &evidence,
        &published,
        key("claim-A")?,
    )?)?];
    let mut input = profile("2026-10-10")?.input().clone();
    input.acquisition_window = ProfileDateRangeV1::new(day("2026-08-02")?, day("2026-10-31")?)?;
    let profile = QueryProfileV1::new(input)?;
    let mut request = request(&fixture, &profile)?;
    request.acquisition_cutoff = instant("2026-08-02T12:00:00Z")?;
    let assessed = assess_current_affiliation(request, inventory(request, &assertions, &[]))?;
    check!(eq; assessed.outcome, CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school));
    check!(eq; assessed.facts.len(), 1);
    Ok(())
}

#[test]
fn existing_evidence_cutoff_and_revision_checks_remain_canonical() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let raw = fixture.assertion(&evidence, &published, key("claim-A")?)?;
    let assertions = [LocallyCheckedAssertion::check(raw)?];
    let profile = profile("2026-10-10")?;
    let mut request = request(&fixture, &profile)?;
    request.acquisition_cutoff = instant("2026-08-01T11:59:59Z")?;
    check!(matches!(
        assess_current_affiliation(request, inventory(request, &assertions, &[])),
        Err(CurrentAffiliationError::Candidate(
            AffiliationCandidateError::AcquisitionAfterDeclaredCutoff
        ))
    ));
    let mut raw = raw;
    raw.owner.known = known(2, 3)?;
    let assertions = [LocallyCheckedAssertion::check(raw)?];
    request.acquisition_cutoff = instant("2026-08-01T12:00:00Z")?;
    check!(matches!(
        assess_current_affiliation(request, inventory(request, &assertions, &[])),
        Err(CurrentAffiliationError::Candidate(
            AffiliationCandidateError::DependencyNotVisible
        ))
    ));
    Ok(())
}

#[test]
fn material_admission_still_rejects_inventory_larger_than_the_grant() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let assertions = [
        LocallyCheckedAssertion::check(fixture.assertion(
            &evidence,
            &published,
            key("claim-A")?,
        )?)?,
        LocallyCheckedAssertion::check(fixture.assertion(
            &evidence,
            &published,
            key("claim-B")?,
        )?)?,
    ];
    let profile = profile("2026-10-10")?;
    let mut request = request(&fixture, &profile)?;
    request.grant = SelectionGrant::new(
        fixture::size(1)?,
        fixture::size(1)?,
        fixture::size(2)?,
        fixture::size(8_388_608)?,
        fixture::size(2_000_000)?,
    )?;
    check!(matches!(
        assess_current_affiliation(request, inventory(request, &assertions, &[])),
        Err(CurrentAffiliationError::Candidate(
            AffiliationCandidateError::BudgetExceeded {
                resource: Resource::Assertions,
                needed: 2,
                maximum: 1
            }
        ))
    ));
    Ok(())
}
