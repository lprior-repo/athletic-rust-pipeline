use super::{fixture::*, TestResult};
use crate::affiliation_candidates::*;
use crate::model::SourceCapture;

#[test]
fn separately_allocated_replay_material_is_charged_before_full_equality() -> TestResult {
    let fixture = Fixture::new()?;
    let first = fixture.evidence()?;
    let copied = BODY.to_vec();
    let mut raw = *first.raw();
    raw.body = &copied;
    let second = LocallyCheckedEvidence::check(raw)?;
    let owners = [&fixture.owner];
    let a = LocallyCheckedAssertion::check(fixture.assertion(&first, &owners, key("same")?)?)?;
    let b = LocallyCheckedAssertion::check(fixture.assertion(&second, &owners, key("same")?)?)?;
    let facts = [a, b];
    let mut query = fixture.query()?;
    let expected = BODY
        .len()
        .checked_mul(3)
        .and_then(|n| n.checked_add(20))
        .ok_or("test arithmetic")?;
    query.grant = SelectionGrant::new(size(2)?, size(1)?, size(2)?, size(8_388_608)?, size(20)?)?;
    check!(matches!(select_candidates(query, &facts, &[]),
        Err(AffiliationCandidateError::BudgetExceeded { resource: Resource::WorkUnits, needed, maximum: 20 }) if needed == expected));
    query.grant = SelectionGrant::new(
        size(2)?,
        size(1)?,
        size(2)?,
        size(8_388_608)?,
        size(expected)?,
    )?;
    let selected = select_candidates(query, &facts, &[])?;
    check!(eq; selected.facts.len(), 1);
    check!(eq; selected.outcome, CandidateOutcome::SingleCandidate(&a));
    Ok(())
}

#[test]
fn shared_immutable_replay_bytes_do_not_consume_material_comparison_grant() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let a = LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("same")?)?)?;
    let facts = [a, a];
    let mut query = fixture.query()?;
    query.grant = SelectionGrant::new(size(2)?, size(1)?, size(2)?, size(8_388_608)?, size(20)?)?;
    let selected = select_candidates(query, &facts, &[])?;
    check!(eq; selected.facts.len(), 1);
    check!(eq; selected.outcome, CandidateOutcome::SingleCandidate(&a));
    Ok(())
}

#[test]
fn legacy_whitespace_date_is_bounded_before_candidate_date_parsing() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let date = format!("{}2026-08-01", " ".repeat(65));
    let metadata = SourceCapture::new(
        &fixture.owner,
        fixture.metadata.body_hash().to_owned(),
        fixture.metadata.byte_len(),
        date,
        Some("https://roster.example.test/requested".to_owned()),
    )?;
    let mut raw = *evidence.raw();
    raw.metadata = &metadata;
    check!(matches!(
        LocallyCheckedEvidence::check(raw),
        Err(AffiliationCandidateError::BudgetExceeded {
            resource: Resource::MetadataBytes,
            needed: 75,
            maximum: 64
        })
    ));
    Ok(())
}

#[test]
fn distinct_assertions_charge_both_capture_and_interpretation_material_checks() -> TestResult {
    let fixture = Fixture::new()?;
    let first = fixture.evidence()?;
    let copied = BODY.to_vec();
    let mut raw = *first.raw();
    raw.body = &copied;
    let second = LocallyCheckedEvidence::check(raw)?;
    let owners = [&fixture.owner];
    let a = LocallyCheckedAssertion::check(fixture.assertion(&first, &owners, key("A")?)?)?;
    let b = LocallyCheckedAssertion::check(fixture.assertion(&second, &owners, key("B")?)?)?;
    let facts = [a, b];
    let expected = BODY
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(20))
        .ok_or("test arithmetic")?;
    let lower = expected.checked_sub(1).ok_or("test arithmetic")?;
    let mut query = fixture.query()?;
    query.grant =
        SelectionGrant::new(size(2)?, size(1)?, size(2)?, size(8_388_608)?, size(lower)?)?;
    check!(matches!(select_candidates(query, &facts, &[]),
        Err(AffiliationCandidateError::BudgetExceeded { resource: Resource::WorkUnits, needed, maximum }) if needed == expected && maximum == lower));
    query.grant = SelectionGrant::new(
        size(2)?,
        size(1)?,
        size(2)?,
        size(8_388_608)?,
        size(expected)?,
    )?;
    let selected = select_candidates(query, &facts, &[])?;
    check!(eq; selected.outcome, CandidateOutcome::SingleCandidate(&a));
    check!(eq; selected.facts,
    [
        FactDisposition::Assertion {
            fact: &a,
            disposition: Disposition::Candidate
        },
        FactDisposition::Assertion {
            fact: &b,
            disposition: Disposition::Candidate
        }
    ]);
    Ok(())
}

#[test]
fn same_address_different_length_is_not_equal_material() -> TestResult {
    let fixture = Fixture::new()?;
    let first = fixture.evidence()?;
    let mut raw = *first.raw();
    let shorter = raw
        .interpretation_bytes
        .len()
        .checked_sub(1)
        .ok_or("test arithmetic")?;
    raw.interpretation_bytes = raw
        .interpretation_bytes
        .get(..shorter)
        .ok_or("test slice")?;
    let second = LocallyCheckedEvidence::check(raw)?;
    let owners = [&fixture.owner];
    let a = LocallyCheckedAssertion::check(fixture.assertion(&first, &owners, key("A")?)?)?;
    let b = LocallyCheckedAssertion::check(fixture.assertion(&second, &owners, key("B")?)?)?;
    let facts = [a, b];
    let mut query = fixture.query()?;
    query.grant = SelectionGrant::new(size(2)?, size(1)?, size(2)?, size(8_388_608)?, size(20)?)?;
    check!(matches!(
        select_candidates(query, &facts, &[]),
        Err(AffiliationCandidateError::DuplicateIdDifferentPayload)
    ));
    Ok(())
}

#[test]
fn legacy_capture_date_exact_candidate_limit_is_preserved() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    for spaces in [54, 55] {
        let date = format!("{}2026-08-01", " ".repeat(spaces));
        let metadata = SourceCapture::new(
            &fixture.owner,
            fixture.metadata.body_hash().to_owned(),
            fixture.metadata.byte_len(),
            date.clone(),
            Some("https://roster.example.test/requested".to_owned()),
        )?;
        let mut raw = *evidence.raw();
        raw.metadata = &metadata;
        let checked = LocallyCheckedEvidence::check(raw);
        if spaces == 54 {
            check!(eq; checked?.raw().metadata.captured_at(), date);
        } else {
            check!(matches!(
                checked,
                Err(AffiliationCandidateError::BudgetExceeded {
                    resource: Resource::MetadataBytes,
                    needed: 65,
                    maximum: 64
                })
            ));
        }
    }
    Ok(())
}
