use super::{fixture::*, TestResult};
use crate::affiliation_candidates::*;
use crate::model::{ProfileDateRangeV1, SchoolYear, Sport};

fn boundary<'a>(
    mut query: CandidateQuery<'a>,
    facts: &'a [LocallyCheckedAssertion<'a>],
    changes: &'a [LocallyCheckedChange<'a>],
    work: usize,
) -> TestResult<CandidateSelection<'a>> {
    let lower = work.checked_sub(1).ok_or("synthetic work")?;
    query.grant = SelectionGrant::new(
        size(256)?,
        size(512)?,
        size(768)?,
        size(8_388_608)?,
        size(lower)?,
    )?;
    assert!(
        matches!(select_candidates(query, facts, changes), Err(AffiliationCandidateError::BudgetExceeded { resource: Resource::WorkUnits, needed, maximum }) if needed == work && maximum == lower)
    );
    query.grant = SelectionGrant::new(
        size(256)?,
        size(512)?,
        size(768)?,
        size(8_388_608)?,
        size(work)?,
    )?;
    Ok(select_candidates(query, facts, changes)?)
}

#[test]
fn duplicate_change_material_is_admitted_before_replay_equality() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let bytes = BODY.to_vec();
    let mut raw = *evidence.raw();
    raw.body = &bytes;
    let copied = LocallyCheckedEvidence::check(raw)?;
    let owners = [&fixture.owner];
    let fact = LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("A")?)?)?;
    let outside = ProfileDateRangeV1::new(day("2026-01-01")?, day("2026-01-02")?)?;
    let change = fixture.change(
        &evidence,
        key("A")?,
        ChangeKind::WithdrawsClaim { scope: &outside },
    )?;
    let mut other = *change.raw();
    other.evidence = &copied;
    let changes = [change, LocallyCheckedChange::check(other)?];
    let facts = [fact];
    let work = BODY
        .len()
        .checked_mul(5)
        .and_then(|n| n.checked_add(25))
        .ok_or("synthetic work")?;
    let selected = boundary(fixture.query()?, &facts, &changes, work)?;
    assert_eq!(selected.outcome, CandidateOutcome::SingleCandidate(&fact));
    assert_eq!(
        selected.facts,
        [
            FactDisposition::Assertion {
                fact: &fact,
                disposition: Disposition::Candidate
            },
            FactDisposition::Change {
                fact: &change,
                disposition: Disposition::ChangeOutsideEffectiveScope
            }
        ]
    );
    Ok(())
}

#[test]
fn nonshared_manifest_and_interpretation_have_separate_comparison_charges() -> TestResult {
    let fixture = Fixture::new()?;
    let first = fixture.evidence()?;
    let manifest = first.raw().manifest_bytes.to_vec();
    let interpretation = first.raw().interpretation_bytes.to_vec();
    let mut raw = *first.raw();
    raw.manifest_bytes = &manifest;
    raw.interpretation_bytes = &interpretation;
    let second = LocallyCheckedEvidence::check(raw)?;
    let owners = [&fixture.owner];
    let a = LocallyCheckedAssertion::check(fixture.assertion(&first, &owners, key("A")?)?)?;
    let b = LocallyCheckedAssertion::check(fixture.assertion(&second, &owners, key("B")?)?)?;
    let facts = [a, b];
    let work = manifest
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(interpretation.len()))
        .and_then(|n| n.checked_add(20))
        .ok_or("synthetic work")?;
    let selected = boundary(fixture.query()?, &facts, &[], work)?;
    assert_eq!(selected.outcome, CandidateOutcome::SingleCandidate(&a));
    assert_eq!(
        selected.facts,
        [
            FactDisposition::Assertion {
                fact: &a,
                disposition: Disposition::Candidate
            },
            FactDisposition::Assertion {
                fact: &b,
                disposition: Disposition::Candidate
            }
        ]
    );
    Ok(())
}

#[test]
fn duplicate_calendar_charges_label_and_nested_evidence_before_deduplication() -> TestResult {
    let fixture = Fixture::new()?;
    let first = fixture.evidence()?;
    let bytes = BODY.to_vec();
    let copied_label = b"fall 2026".to_vec();
    let mut raw = *first.raw();
    raw.body = &bytes;
    let copied = LocallyCheckedEvidence::check(raw)?;
    let owners = [&fixture.owner];
    let year = SchoolYear::new(2026).ok_or("synthetic year")?;
    let calendar = CalendarCandidate {
        year,
        sport: Sport::CrossCountry,
        interval: &fixture.interval,
        raw_label: b"fall 2026",
        label_convention: key("fall")?,
        mapping_rule: &fixture.rule,
        evidence: &first,
        locator: key("calendar")?,
        statement: SourceSpan::new(0, BODY.len())?,
        known: known(1, 3)?,
    };
    for nested_copy in [false, true] {
        let other = CalendarCandidate {
            raw_label: &copied_label,
            evidence: if nested_copy { &copied } else { &first },
            ..calendar
        };
        let mut a = fixture.assertion(&first, &owners, key("A")?)?;
        a.temporal = TemporalScope::AcademicSeason {
            year,
            sport: Sport::CrossCountry,
            calendar: &calendar,
        };
        let b = RawAssertion {
            temporal: TemporalScope::AcademicSeason {
                year,
                sport: Sport::CrossCountry,
                calendar: &other,
            },
            ..a
        };
        let facts = [
            LocallyCheckedAssertion::check(a)?,
            LocallyCheckedAssertion::check(b)?,
        ];
        let nested = if nested_copy {
            BODY.len().checked_mul(7).ok_or("synthetic work")?
        } else {
            0
        };
        let work = 20_usize
            .checked_add(copied_label.len())
            .and_then(|n| n.checked_add(nested))
            .ok_or("synthetic work")?;
        let selected = boundary(fixture.query()?, &facts, &[], work)?;
        let fact = facts.first().ok_or("synthetic fact")?;
        assert_eq!(selected.outcome, CandidateOutcome::SingleCandidate(fact));
        assert_eq!(
            selected.facts,
            [FactDisposition::Assertion {
                fact,
                disposition: Disposition::Candidate
            }]
        );
    }
    Ok(())
}

#[test]
fn equal_length_metadata_requires_exact_bytes_after_admission() -> TestResult {
    let fixture = Fixture::new()?;
    let first = fixture.evidence()?;
    let owners = [&fixture.owner];
    for mutate_manifest in [true, false] {
        let mut manifest = first.raw().manifest_bytes.to_vec();
        let mut interpretation = first.raw().interpretation_bytes.to_vec();
        let target = if mutate_manifest {
            &mut manifest
        } else {
            &mut interpretation
        };
        let last = target.last_mut().ok_or("synthetic metadata absent")?;
        *last = b'Z';
        let mut raw = *first.raw();
        raw.manifest_bytes = &manifest;
        raw.interpretation_bytes = &interpretation;
        let second = LocallyCheckedEvidence::check(raw)?;
        let facts = [
            LocallyCheckedAssertion::check(fixture.assertion(&first, &owners, key("A")?)?)?,
            LocallyCheckedAssertion::check(fixture.assertion(&second, &owners, key("B")?)?)?,
        ];
        assert!(matches!(
            select_candidates(fixture.query()?, &facts, &[]),
            Err(AffiliationCandidateError::DuplicateIdDifferentPayload)
        ));
    }
    Ok(())
}
