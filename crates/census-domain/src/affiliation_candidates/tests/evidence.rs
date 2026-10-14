use super::{fixture::*, TestResult};
use crate::affiliation_candidates::*;

#[test]
fn bare_digest_or_journal_skipped_body_cannot_supply_material() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let mut raw = *evidence.raw();
    raw.body = &[];
    assert_eq!(
        LocallyCheckedEvidence::check(raw),
        Err(AffiliationCandidateError::MissingCaptureMaterial)
    );
    raw = *evidence.raw();
    raw.manifest_bytes = &[];
    assert_eq!(
        LocallyCheckedEvidence::check(raw),
        Err(AffiliationCandidateError::MissingCaptureMaterial)
    );
    raw = *evidence.raw();
    raw.interpretation_bytes = &[];
    assert_eq!(
        LocallyCheckedEvidence::check(raw),
        Err(AffiliationCandidateError::MissingInterpretationMaterial)
    );
    Ok(())
}

#[test]
fn supplied_body_must_match_length_and_hash() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let mut raw = *evidence.raw();
    raw.body = b"short";
    assert_eq!(
        LocallyCheckedEvidence::check(raw),
        Err(AffiliationCandidateError::LengthMismatch)
    );
    let replacement = vec![b'x'; BODY.len()];
    raw.body = &replacement;
    assert_eq!(
        LocallyCheckedEvidence::check(raw),
        Err(AffiliationCandidateError::HashMismatch)
    );
    Ok(())
}

#[test]
fn unknown_final_url_and_exact_acquisition_instant_survive() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    assert_eq!(evidence.raw().final_url, FinalUrl::Unknown);
    assert_eq!(evidence.raw().acquired_at, instant("2026-08-01T12:00:00Z")?);
    let mut raw = *evidence.raw();
    raw.final_url = FinalUrl::Observed("https://roster.example.test/observed");
    let observed = LocallyCheckedEvidence::check(raw)?;
    assert_ne!(
        observed.raw().requested_url,
        "https://roster.example.test/observed"
    );
    assert_eq!(observed.raw().final_url, raw.final_url);
    Ok(())
}

#[test]
fn owner_and_actual_date_cannot_be_replaced() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let mut raw = *evidence.raw();
    raw.owner = &fixture.other_owner;
    assert_eq!(
        LocallyCheckedEvidence::check(raw),
        Err(AffiliationCandidateError::ForeignCaptureOwner)
    );
    raw = *evidence.raw();
    raw.acquired_at = instant("2026-08-02T00:00:00Z")?;
    assert_eq!(
        LocallyCheckedEvidence::check(raw),
        Err(AffiliationCandidateError::AcquisitionDateMismatch)
    );
    Ok(())
}

#[test]
fn empty_reversed_and_out_of_body_spans_refuse() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    assert_eq!(
        SourceSpan::new(3, 3),
        Err(AffiliationCandidateError::InvalidLocator)
    );
    assert_eq!(
        SourceSpan::new(4, 3),
        Err(AffiliationCandidateError::InvalidLocator)
    );
    assert_eq!(
        SourceSpan::new(0, 4097),
        Err(AffiliationCandidateError::InvalidLocator)
    );
    assert_eq!(
        SourceSpan::new(0, 1000)?.read(&evidence),
        Err(AffiliationCandidateError::InvalidLocator)
    );
    Ok(())
}

#[test]
fn contradictory_governing_owner_refuses_locally() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner, &fixture.other_owner];
    let raw = fixture.assertion(&evidence, &published, key("synthetic-claim-A")?)?;
    assert_eq!(
        LocallyCheckedAssertion::check(raw),
        Err(AffiliationCandidateError::ConflictingAuthoritativeOwners)
    );
    Ok(())
}
