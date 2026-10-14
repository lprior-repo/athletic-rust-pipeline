use super::{fixture::*, TestResult};
use crate::affiliation_candidates::*;

#[test]
fn resource_exhaustion_never_returns_a_preferred_prefix() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let a =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-A")?)?)?;
    let b =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-B")?)?)?;
    let claims = [a, b];
    let mut query = fixture.query()?;
    query.grant = SelectionGrant::new(
        size(1)?,
        size(512)?,
        size(768)?,
        size(8_388_608)?,
        size(2_000_000)?,
    )?;
    assert!(matches!(
        select_candidates(query, &claims, &[]),
        Err(AffiliationCandidateError::BudgetExceeded {
            resource: Resource::Assertions,
            needed: 2,
            maximum: 1
        })
    ));
    query.grant = SelectionGrant::new(
        size(256)?,
        size(512)?,
        size(1)?,
        size(8_388_608)?,
        size(2_000_000)?,
    )?;
    assert!(matches!(
        select_candidates(query, &claims, &[]),
        Err(AffiliationCandidateError::BudgetExceeded {
            resource: Resource::AuditFacts,
            ..
        })
    ));
    query.grant = SelectionGrant::new(
        size(256)?,
        size(512)?,
        size(768)?,
        size(1)?,
        size(2_000_000)?,
    )?;
    assert!(matches!(
        select_candidates(query, &claims, &[]),
        Err(AffiliationCandidateError::BudgetExceeded {
            resource: Resource::ComponentBytes,
            ..
        })
    ));
    Ok(())
}

#[test]
fn reparse_keeps_one_capture_and_distinct_interpretations() -> TestResult {
    let fixture = Fixture::new()?;
    let first_evidence = fixture.evidence()?;
    let mut second_raw = *first_evidence.raw();
    second_raw.interpretation_key = key("synthetic-interpretation-B")?;
    second_raw.interpretation_bytes = b"synthetic extraction record B";
    second_raw.parser_artifact = key("synthetic-parser-B")?;
    let second_evidence = LocallyCheckedEvidence::check(second_raw)?;
    let owners = [&fixture.owner];
    let first = LocallyCheckedAssertion::check(fixture.assertion(
        &first_evidence,
        &owners,
        key("claim-A")?,
    )?)?;
    let second = LocallyCheckedAssertion::check(fixture.assertion(
        &second_evidence,
        &owners,
        key("claim-B")?,
    )?)?;
    let claims = [first, second];
    let result = select_candidates(fixture.query()?, &claims, &[])?;
    assert_eq!(result.outcome, CandidateOutcome::SingleCandidate(&first));
    assert_eq!(result.facts.len(), 2);
    let material = first_evidence.raw();
    let exact_bytes = material
        .body
        .len()
        .checked_add(material.manifest_bytes.len())
        .and_then(|n| n.checked_add(material.interpretation_bytes.len()))
        .and_then(|n| n.checked_add(second_raw.interpretation_bytes.len()))
        .ok_or("synthetic size overflow")?;
    assert_component_boundary(fixture.query()?, &claims, exact_bytes)?;
    assert_eq!(
        first.raw().evidence.raw().occurrence,
        second.raw().evidence.raw().occurrence
    );
    assert_ne!(
        first.raw().evidence.raw().interpretation_key,
        second.raw().evidence.raw().interpretation_key
    );
    Ok(())
}

#[test]
fn same_bytes_later_actual_occurrence_is_not_suppressed_by_legacy_id() -> TestResult {
    let fixture = Fixture::new()?;
    let first_evidence = fixture.evidence()?;
    let mut second_raw = *first_evidence.raw();
    second_raw.occurrence = key("synthetic-occurrence-B")?;
    second_raw.manifest_key = key("synthetic-manifest-B")?;
    second_raw.manifest_bytes = b"synthetic manifest B";
    second_raw.interpretation_key = key("synthetic-interpretation-B")?;
    second_raw.interpretation_bytes = b"synthetic extraction record B";
    second_raw.acquired_at = instant("2026-08-01T13:00:00Z")?;
    let second_evidence = LocallyCheckedEvidence::check(second_raw)?;
    let owners = [&fixture.owner];
    let first = LocallyCheckedAssertion::check(fixture.assertion(
        &first_evidence,
        &owners,
        key("claim-A")?,
    )?)?;
    let second = LocallyCheckedAssertion::check(fixture.assertion(
        &second_evidence,
        &owners,
        key("claim-B")?,
    )?)?;
    let claims = [first, second];
    let mut query = fixture.query()?;
    query.acquisition_cutoff = instant("2026-08-01T13:00:00Z")?;
    let result = select_candidates(query, &claims, &[])?;
    assert_eq!(result.facts.len(), 2);
    let material = first_evidence.raw();
    let exact_bytes = material
        .body
        .len()
        .checked_add(material.manifest_bytes.len())
        .and_then(|n| n.checked_add(material.interpretation_bytes.len()))
        .and_then(|n| n.checked_add(second_raw.body.len()))
        .and_then(|n| n.checked_add(second_raw.manifest_bytes.len()))
        .and_then(|n| n.checked_add(second_raw.interpretation_bytes.len()))
        .ok_or("synthetic size overflow")?;
    assert_component_boundary(query, &claims, exact_bytes)?;
    assert_eq!(
        first_evidence.raw().metadata.capture_id(),
        second_evidence.raw().metadata.capture_id()
    );
    assert_ne!(
        first_evidence.raw().occurrence,
        second_evidence.raw().occurrence
    );
    assert_ne!(
        first_evidence.raw().acquired_at,
        second_evidence.raw().acquired_at
    );
    Ok(())
}

#[test]
fn same_interpretation_reference_cannot_hide_changed_parser_or_capture() -> TestResult {
    let fixture = Fixture::new()?;
    let first_evidence = fixture.evidence()?;
    let mut second_raw = *first_evidence.raw();
    second_raw.parser_artifact = key("changed-parser-with-reused-interpretation-key")?;
    let second_evidence = LocallyCheckedEvidence::check(second_raw)?;
    let owners = [&fixture.owner];
    let claims = [
        LocallyCheckedAssertion::check(fixture.assertion(
            &first_evidence,
            &owners,
            key("claim-A")?,
        )?)?,
        LocallyCheckedAssertion::check(fixture.assertion(
            &second_evidence,
            &owners,
            key("claim-B")?,
        )?)?,
    ];
    assert!(matches!(
        select_candidates(fixture.query()?, &claims, &[]),
        Err(AffiliationCandidateError::DuplicateIdDifferentPayload)
    ));
    Ok(())
}

#[test]
fn late_acquisition_and_impossible_frozen_instant_refuse() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let claims = [LocallyCheckedAssertion::check(fixture.assertion(
        &evidence,
        &owners,
        key("claim-A")?,
    )?)?];
    let mut query = fixture.query()?;
    query.acquisition_cutoff = instant("2026-08-01T11:59:59Z")?;
    assert!(matches!(
        select_candidates(query, &claims, &[]),
        Err(AffiliationCandidateError::AcquisitionAfterDeclaredCutoff)
    ));
    query = fixture.query()?;
    query.known.captured_at = instant("2026-08-01T11:59:59Z")?;
    assert!(matches!(
        select_candidates(query, &claims, &[]),
        Err(AffiliationCandidateError::AcquisitionAfterFrozenInstant)
    ));
    query = fixture.query()?;
    query.acquisition_cutoff = instant("2026-10-11T00:00:00Z")?;
    assert!(matches!(
        select_candidates(query, &[], &[]),
        Err(AffiliationCandidateError::CurrentCompilerEnvelopeViolated)
    ));
    Ok(())
}

#[test]
fn same_school_enrollment_and_program_remain_distinct_relationship_candidates() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let a =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-A")?)?)?;
    let mut b = fixture.assertion(&evidence, &owners, key("claim-B")?)?;
    b.relationship = RelationshipCandidate::RepresentsSchoolProgram {
        school: &fixture.school,
        program: &fixture.program,
    };
    let claims = [a, LocallyCheckedAssertion::check(b)?];
    let result = select_candidates(fixture.query()?, &claims, &[])?;
    assert!(matches!(
        result.outcome,
        CandidateOutcome::MultipleCompatibleCandidates
    ));
    assert_eq!(result.facts.len(), 2);
    Ok(())
}

fn assert_component_boundary<'a>(
    mut query: CandidateQuery<'a>,
    claims: &'a [LocallyCheckedAssertion<'a>],
    exact: usize,
) -> TestResult {
    let lower = exact.checked_sub(1).ok_or("synthetic size underflow")?;
    query.grant = SelectionGrant::new(
        size(256)?,
        size(512)?,
        size(768)?,
        size(lower)?,
        size(2_000_000)?,
    )?;
    assert!(
        matches!(select_candidates(query, claims, &[]), Err(AffiliationCandidateError::BudgetExceeded { resource: Resource::ComponentBytes, needed, maximum }) if needed == exact && maximum == lower)
    );
    query.grant = SelectionGrant::new(
        size(256)?,
        size(512)?,
        size(768)?,
        size(exact)?,
        size(2_000_000)?,
    )?;
    let selected = select_candidates(query, claims, &[])?;
    assert_eq!(selected.facts.len(), claims.len());
    assert_eq!(
        selected.outcome,
        CandidateOutcome::SingleCandidate(claims.first().ok_or("synthetic claim absent")?)
    );
    Ok(())
}

#[test]
fn reused_interpretation_key_refuses_changed_occurrence_and_final_url() -> TestResult {
    let fixture = Fixture::new()?;
    let first = fixture.evidence()?;
    let owners = [&fixture.owner];
    for changed_occurrence in [true, false] {
        let mut raw = *first.raw();
        if changed_occurrence {
            raw.occurrence = key("different-occurrence")?;
            raw.manifest_key = key("different-manifest")?;
        } else {
            raw.final_url = FinalUrl::Observed("https://roster.example.test/redirected");
        }
        let second = LocallyCheckedEvidence::check(raw)?;
        let claims = [
            LocallyCheckedAssertion::check(fixture.assertion(&first, &owners, key("A")?)?)?,
            LocallyCheckedAssertion::check(fixture.assertion(&second, &owners, key("B")?)?)?,
        ];
        assert!(matches!(
            select_candidates(fixture.query()?, &claims, &[]),
            Err(AffiliationCandidateError::DuplicateIdDifferentPayload)
        ));
    }
    Ok(())
}
