use super::super::{fixture::Fixture, AffiliationCandidateError, OwnerSourceSpan, SourceSpan};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn owner_source_span_zero_length_is_invalid() -> TestResult {
    check!(eq; OwnerSourceSpan::new(10, 10), Err(AffiliationCandidateError::InvalidLocator));
    Ok(())
}

#[test]
fn owner_source_span_reversed_bounds_is_invalid() -> TestResult {
    check!(eq; OwnerSourceSpan::new(10, 5), Err(AffiliationCandidateError::InvalidLocator));
    Ok(())
}

#[test]
fn owner_source_span_exceeding_its_maximum_is_invalid() -> TestResult {
    let end = OwnerSourceSpan::MAX_BYTES
        .checked_add(1)
        .ok_or("owner boundary overflow")?;
    check!(eq; OwnerSourceSpan::new(0, end), Err(AffiliationCandidateError::InvalidLocator));
    Ok(())
}

#[test]
fn owner_source_span_at_its_maximum_is_admitted() -> TestResult {
    let span = OwnerSourceSpan::new(0, OwnerSourceSpan::MAX_BYTES)?;
    check!(eq; span.range(), 0..OwnerSourceSpan::MAX_BYTES);
    Ok(())
}

#[test]
fn owner_source_span_above_the_ordinary_limit_is_admitted() -> TestResult {
    let end = SourceSpan::MAX_BYTES
        .checked_add(1)
        .ok_or("ordinary boundary overflow")?;
    check!(eq; OwnerSourceSpan::new(0, end)?.range(), 0..end);
    check!(eq; SourceSpan::new(0, end), Err(AffiliationCandidateError::InvalidLocator));
    Ok(())
}

#[test]
fn owner_source_span_read_outside_evidence_body_is_invalid() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let span = OwnerSourceSpan::new(0, evidence.raw().body.len())?;
    check!(eq; span.read(&evidence)?, evidence.raw().body);
    let end = evidence
        .raw()
        .body
        .len()
        .checked_add(1)
        .ok_or("body boundary overflow")?;
    let overreaching = OwnerSourceSpan::new(0, end)?;
    check!(eq; overreaching.read(&evidence), Err(AffiliationCandidateError::InvalidLocator));
    Ok(())
}
