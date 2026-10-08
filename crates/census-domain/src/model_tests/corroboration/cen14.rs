use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn cen14_independent_captures_accept_in_both_orders() -> TestResult {
    let mut rows = independent_pair()?;
    check!(admits(&rows)?);
    rows.reverse();
    check!(admits(&rows)?);
    Ok(())
}

#[test]
fn cen14_mirrored_urls_same_capture_never_corroborate_in_both_orders() -> TestResult {
    let [first, mut second] = independent_pair()?;
    let digest = first
        .identity_attestations
        .first()
        .ok_or("first capture")?
        .capture_sha256
        .clone();
    second
        .identity_attestations
        .first_mut()
        .ok_or("second capture")?
        .capture_sha256 = digest.to_uppercase();
    let mut rows = [first, second];
    check!(!admits(&rows)?);
    rows.reverse();
    check!(!admits(&rows)?);
    Ok(())
}

#[test]
fn cen14_different_captures_same_producer_never_corroborate_in_both_orders() -> TestResult {
    let [first, mut second] = independent_pair()?;
    let producer = first
        .identity_attestations
        .first()
        .ok_or("first capture")?
        .upstream_producer
        .clone();
    second
        .identity_attestations
        .first_mut()
        .ok_or("second capture")?
        .upstream_producer = producer.to_uppercase();
    let mut rows = [first, second];
    check!(!admits(&rows)?);
    rows.reverse();
    check!(!admits(&rows)?);
    Ok(())
}

#[test]
fn cen14_source_family_aliases_and_candidates_do_not_corroborate() -> TestResult {
    let [first, mut second] = independent_pair()?;
    let family = first
        .identity_attestations
        .first()
        .ok_or("first capture")?
        .source_family
        .clone();
    second
        .identity_attestations
        .first_mut()
        .ok_or("second capture")?
        .source_family = family.to_uppercase();
    check!(!admits(&[first, second])?);
    let mut rows = independent_pair()?;
    for row in &mut rows {
        for claim in &mut row.identity_attestations {
            claim.qualification = AttestationQualification::CandidateOnly;
        }
    }
    check!(!admits(&rows)?);
    Ok(())
}

#[test]
fn cen14_unbound_claim_is_an_error_without_registering_the_subject() -> TestResult {
    let [mut row, _] = independent_pair()?;
    row.identity_attestations
        .first_mut()
        .ok_or("capture")?
        .subject = tfrrs("777");
    let mut index = AthleteIdentityIndex::default();
    check!(
        matches!(index.observe(&row), Err(IdentityError::UnboundAttestation(id)) if id == "777")
    );
    check!(eq; index.subjects().count(), 0);
    Ok(())
}

#[test]
fn cen14_typed_claim_serialization_preserves_independence_and_refuses_invalid_capture() -> TestResult
{
    let rows = independent_pair()?;
    let bytes = serde_json::to_vec(&rows)?;
    let reopened: [CanonicalAthlete; 2] = serde_json::from_slice(&bytes)?;
    check!(eq; reopened, rows);
    check!(admits(&reopened)?);
    let mut claim = captured_claim(&tfrrs("999"), "roster-fixture", "school-fixture")?;
    claim.capture_sha256 = "not-a-capture".into();
    check!(eq; claim.validate(), Err(AttestationError::InvalidCapture));
    let rejected = serde_json::from_slice::<IdentityAttestation>(&serde_json::to_vec(&claim)?);
    check!(
        rejected.is_err(),
        "invalid immutable capture must not deserialize"
    );
    Ok(())
}

#[test]
fn cen14_credentialed_or_missing_capture_url_cannot_qualify() -> TestResult {
    let mut claim = captured_claim(&tfrrs("999"), "roster-fixture", "school-fixture")?;
    for url in [
        Some("https://user:password@example.test/doc"),
        Some("https://"),
        None,
    ] {
        claim.source.url = url.map(str::to_owned);
        check!(eq; claim.validate(), Err(AttestationError::InvalidCapture));
    }
    Ok(())
}
