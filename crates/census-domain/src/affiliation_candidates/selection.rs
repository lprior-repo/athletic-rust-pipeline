use super::bounds::check_limit;
use super::{
    AffiliationCandidateError, CandidateSelection, FactDisposition, LocallyCheckedAssertion,
    LocallyCheckedChange, ObjectRef, Resource, SelectionGrant, TemporalScope,
};
use crate::model::{AthleteId, ContractRefV1, ProfileDate, Sport};
use chrono::{DateTime, NaiveDate, Utc};
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclaredFrozenView<'a> {
    pub lineage: ObjectRef<'a>,
    pub revision: NonZeroU64,
    pub captured_at: DateTime<Utc>,
    pub closure_object: ObjectRef<'a>,
    pub family_inventory: ObjectRef<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateQuery<'a> {
    pub athlete: &'a AthleteId,
    pub as_of: &'a ProfileDate,
    pub sport: Sport,
    pub known: DeclaredFrozenView<'a>,
    pub acquisition_cutoff: DateTime<Utc>,
    pub rule_claim: &'a ContractRefV1,
    pub grant: SelectionGrant,
}

pub fn select_candidates<'a>(
    query: CandidateQuery<'a>,
    assertions: &'a [LocallyCheckedAssertion<'a>],
    changes: &'a [LocallyCheckedChange<'a>],
) -> Result<CandidateSelection<'a>, AffiliationCandidateError> {
    select_with_envelope(query, assertions, changes, TemporalEnvelope::LegacyCompiler)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TemporalEnvelope {
    LegacyCompiler,
    IndependentEffectiveAndKnownTime,
}

pub(super) fn select_with_envelope<'a>(
    query: CandidateQuery<'a>,
    assertions: &'a [LocallyCheckedAssertion<'a>],
    changes: &'a [LocallyCheckedChange<'a>],
    envelope: TemporalEnvelope,
) -> Result<CandidateSelection<'a>, AffiliationCandidateError> {
    let pair_work = check_query(&query, assertions.len(), changes.len(), envelope)?;
    super::material_work::check(&query, assertions, changes, pair_work)?;
    let mut positive = copy_refs(assertions)?;
    let mut negative = copy_refs(changes)?;
    positive.sort_unstable_by_key(|fact| fact.raw().id);
    negative.sort_unstable_by_key(|fact| fact.raw().id);
    check_duplicates(&positive, |fact| fact.raw().id)?;
    check_duplicates(&negative, |fact| fact.raw().id)?;
    positive.dedup_by(|left, right| left.raw().id == right.raw().id);
    negative.dedup_by(|left, right| left.raw().id == right.raw().id);
    check_inputs(&positive, &negative, &query)?;
    let fact_count = positive
        .len()
        .checked_add(negative.len())
        .ok_or(AffiliationCandidateError::CounterOverflow)?;
    let mut facts = Vec::new();
    facts
        .try_reserve_exact(fact_count)
        .map_err(|_| AffiliationCandidateError::AllocationFailed)?;
    for fact in positive {
        let disposition = super::selection_reduce::classify_assertion(fact, &negative, &query)?;
        facts.push(FactDisposition::Assertion { fact, disposition });
    }
    for fact in negative {
        let disposition = super::selection_reduce::classify_change(fact, assertions, &query)?;
        facts.push(FactDisposition::Change { fact, disposition });
    }
    let outcome = super::selection_reduce::reduce(&facts)?;
    Ok(CandidateSelection {
        query,
        outcome,
        facts,
    })
}

fn check_query(
    query: &CandidateQuery<'_>,
    assertions: usize,
    changes: usize,
    envelope: TemporalEnvelope,
) -> Result<usize, AffiliationCandidateError> {
    if query.sport == Sport::Unknown {
        return Err(AffiliationCandidateError::UnknownSport);
    }
    if query.athlete.as_str().is_empty() || query.athlete.as_str().len() > 512 {
        return Err(AffiliationCandidateError::InvalidExistingId);
    }
    let effective_day = NaiveDate::parse_from_str(query.as_of.as_str(), "%Y-%m-%d")
        .map_err(|_| AffiliationCandidateError::CurrentCompilerEnvelopeViolated)?;
    if envelope == TemporalEnvelope::LegacyCompiler
        && query.acquisition_cutoff.date_naive() > effective_day
    {
        return Err(AffiliationCandidateError::CurrentCompilerEnvelopeViolated);
    }
    check_limit(Resource::Assertions, assertions, query.grant.assertions)?;
    check_limit(Resource::Changes, changes, query.grant.changes)?;
    let facts = assertions
        .checked_add(changes)
        .ok_or(AffiliationCandidateError::CounterOverflow)?;
    check_limit(Resource::AuditFacts, facts, query.grant.facts)?;
    let evidence = assertions
        .checked_mul(2)
        .and_then(|a| a.checked_add(changes))
        .ok_or(AffiliationCandidateError::CounterOverflow)?;
    let work = facts
        .checked_mul(facts)
        .and_then(|a| {
            evidence
                .checked_mul(evidence)
                .and_then(|b| a.checked_add(b))
        })
        .ok_or(AffiliationCandidateError::CounterOverflow)?;
    check_limit(Resource::WorkUnits, work, query.grant.work)?;
    Ok(work)
}

fn copy_refs<T>(items: &[T]) -> Result<Vec<&T>, AffiliationCandidateError> {
    let mut copied = Vec::new();
    copied
        .try_reserve_exact(items.len())
        .map_err(|_| AffiliationCandidateError::AllocationFailed)?;
    copied.extend(items.iter());
    Ok(copied)
}

fn check_duplicates<'a, T: PartialEq>(
    items: &[&'a T],
    key: impl Fn(&T) -> ObjectRef<'a>,
) -> Result<(), AffiliationCandidateError> {
    for pair in items.windows(2) {
        if let [left, right] = pair {
            if key(left) == key(right) && left != right {
                return Err(AffiliationCandidateError::DuplicateIdDifferentPayload);
            }
        }
    }
    Ok(())
}

fn check_inputs<'a>(
    assertions: &[&'a LocallyCheckedAssertion<'a>],
    changes: &[&'a LocallyCheckedChange<'a>],
    query: &CandidateQuery<'_>,
) -> Result<(), AffiliationCandidateError> {
    let maximum = assertions
        .len()
        .checked_mul(2)
        .and_then(|a| a.checked_add(changes.len()))
        .ok_or(AffiliationCandidateError::CounterOverflow)?;
    let mut seen = Vec::new();
    seen.try_reserve_exact(maximum)
        .map_err(|_| AffiliationCandidateError::AllocationFailed)?;
    let mut bytes = 0;
    for fact in assertions {
        super::selection_checks::check_assertion(fact, query)?;
        super::selection_checks::check_evidence(fact.raw().evidence, &mut seen, &mut bytes, query)?;
        if let TemporalScope::AcademicSeason { calendar, .. } = fact.raw().temporal {
            super::selection_checks::check_evidence(
                calendar.evidence,
                &mut seen,
                &mut bytes,
                query,
            )?;
        }
    }
    for fact in changes {
        super::selection_checks::check_change(fact, assertions, query)?;
        super::selection_checks::check_evidence(fact.raw().evidence, &mut seen, &mut bytes, query)?;
    }
    Ok(())
}
