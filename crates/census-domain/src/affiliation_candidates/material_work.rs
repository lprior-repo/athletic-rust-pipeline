use super::bounds::check_limit;
use super::equality::comparison_bytes;
use super::{
    AffiliationCandidateError, CandidateQuery, LocallyCheckedAssertion, LocallyCheckedChange,
    LocallyCheckedEvidence, RawEvidence, Resource, TemporalScope,
};

pub(super) fn check(
    query: &CandidateQuery<'_>,
    assertions: &[LocallyCheckedAssertion<'_>],
    changes: &[LocallyCheckedChange<'_>],
    pair_work: usize,
) -> Result<(), AffiliationCandidateError> {
    let mut work = pair_work;
    assertion_replay_work(assertions, &mut work)?;
    change_replay_work(changes, &mut work)?;
    evidence_work(assertions, changes, &mut work)?;
    check_limit(Resource::WorkUnits, work, query.grant.work)
}

fn add(work: &mut usize, needed: usize) -> Result<(), AffiliationCandidateError> {
    *work = work
        .checked_add(needed)
        .ok_or(AffiliationCandidateError::CounterOverflow)?;
    Ok(())
}

fn capture_work(
    left: &RawEvidence<'_>,
    right: &RawEvidence<'_>,
    work: &mut usize,
) -> Result<(), AffiliationCandidateError> {
    add(work, comparison_bytes(left.body, right.body))?;
    add(
        work,
        comparison_bytes(left.manifest_bytes, right.manifest_bytes),
    )
}

fn interpretation_work(
    left: &RawEvidence<'_>,
    right: &RawEvidence<'_>,
    work: &mut usize,
) -> Result<(), AffiliationCandidateError> {
    capture_work(left, right, work)?;
    add(
        work,
        comparison_bytes(left.interpretation_bytes, right.interpretation_bytes),
    )
}

fn assertion_replay_work(
    assertions: &[LocallyCheckedAssertion<'_>],
    work: &mut usize,
) -> Result<(), AffiliationCandidateError> {
    for (index, left) in assertions.iter().enumerate() {
        for right in assertions.iter().skip(index).skip(1) {
            let left = left.raw();
            let right = right.raw();
            if left.id != right.id {
                continue;
            }
            interpretation_work(left.evidence.raw(), right.evidence.raw(), work)?;
            if let (
                TemporalScope::AcademicSeason { calendar: left, .. },
                TemporalScope::AcademicSeason {
                    calendar: right, ..
                },
            ) = (left.temporal, right.temporal)
            {
                add(work, comparison_bytes(left.raw_label, right.raw_label))?;
                interpretation_work(left.evidence.raw(), right.evidence.raw(), work)?;
            }
        }
    }
    Ok(())
}

fn change_replay_work(
    changes: &[LocallyCheckedChange<'_>],
    work: &mut usize,
) -> Result<(), AffiliationCandidateError> {
    for (index, left) in changes.iter().enumerate() {
        for right in changes.iter().skip(index).skip(1) {
            if left.raw().id == right.raw().id {
                interpretation_work(left.raw().evidence.raw(), right.raw().evidence.raw(), work)?;
            }
        }
    }
    Ok(())
}

fn evidence_refs<'a>(
    assertions: &'a [LocallyCheckedAssertion<'a>],
    changes: &'a [LocallyCheckedChange<'a>],
) -> impl Iterator<Item = &'a LocallyCheckedEvidence<'a>> + Clone {
    assertions
        .iter()
        .flat_map(|assertion| {
            let raw = assertion.raw();
            let calendar = match raw.temporal {
                TemporalScope::AcademicSeason { calendar, .. } => Some(calendar.evidence),
                _ => None,
            };
            [Some(raw.evidence), calendar].into_iter().flatten()
        })
        .chain(changes.iter().map(|change| change.raw().evidence))
}

fn evidence_work(
    assertions: &[LocallyCheckedAssertion<'_>],
    changes: &[LocallyCheckedChange<'_>],
    work: &mut usize,
) -> Result<(), AffiliationCandidateError> {
    let evidence = evidence_refs(assertions, changes);
    for (index, left) in evidence.clone().enumerate() {
        for right in evidence.clone().skip(index).skip(1) {
            let left = left.raw();
            let right = right.raw();
            if left.occurrence == right.occurrence || left.manifest_key == right.manifest_key {
                capture_work(left, right, work)?;
            }
            if left.interpretation_key == right.interpretation_key {
                interpretation_work(left, right, work)?;
            }
        }
    }
    Ok(())
}
