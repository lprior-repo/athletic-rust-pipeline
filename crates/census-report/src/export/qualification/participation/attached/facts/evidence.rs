use super::*;
use census_crawl::participation_evidence::{
    recorded::{DocumentaryCohortAgreementV1, DocumentarySeasonApplicabilityV1},
    IndependentParticipationProofV1,
};
use census_domain::participation_candidates::{
    ContextFact, EligibilityWindowIdentity, HighSchoolStatusFact, InSeasonWindowFact,
    ParticipationContext, StatusBasis, StudentStatus,
};

pub(super) fn intervals(
    facts: &CheckedDocumentaryParticipationFactsV1<'_>,
    grant: &mut Grant,
) -> ReportResult<Vec<ProfileDateRangeV1>> {
    let count = facts.sources().len();
    let size = std::mem::size_of::<ProfileDateRangeV1>()
        .checked_add(std::mem::size_of::<HighSchoolStatusFact<'_>>())
        .and_then(|n| n.checked_add(std::mem::size_of::<InSeasonWindowFact<'_>>()))
        .and_then(|n| n.checked_add(std::mem::size_of::<ContextFact<'_>>()))
        .and_then(|n| n.checked_add(20))
        .ok_or(census_store::StoreError::CounterOverflow)?;
    grant
        .charge_bytes(
            size.checked_mul(count)
                .and_then(|n| n.checked_mul(4))
                .ok_or(census_store::StoreError::CounterOverflow)?,
        )
        .map_err(domain)?;
    step(grant, count)?;
    let mut intervals = reserved(count)?;
    for item in facts.sources() {
        intervals.push(
            ProfileDateRangeV1::new(item.source().day().clone(), item.source().day().clone())
                .map_err(source)?,
        );
    }
    Ok(intervals)
}

pub(super) fn cohort<'a>(
    facts: &'a CheckedDocumentaryParticipationFactsV1<'a>,
    athlete: &'a AthleteId,
    query: CandidateQuery<'a>,
) -> ReportResult<CohortFact<'a>> {
    let Some(first) = facts.sources().first() else {
        return Ok(CohortFact::Missing);
    };
    if facts.sources().iter().any(|item| {
        item.source().cohort() != first.source().cohort()
            || item.cohort_agreement() == DocumentaryCohortAgreementV1::Conflicting
    }) {
        return Ok(CohortFact::Conflicting);
    }
    Ok(CohortFact::Resolved {
        year: first.source().cohort(),
        proof: proof(first.source().status_proof(), athlete, query)?,
    })
}

pub(super) fn statuses<'a>(
    facts: &'a CheckedDocumentaryParticipationFactsV1<'a>,
    intervals: &'a [ProfileDateRangeV1],
    athlete: &'a AthleteId,
    query: CandidateQuery<'a>,
) -> ReportResult<Vec<HighSchoolStatusFact<'a>>> {
    let mut output = reserved(facts.sources().len())?;
    for (item, interval) in facts.sources().iter().zip(intervals) {
        output.push(HighSchoolStatusFact {
            status: StudentStatus::HighSchool,
            basis: StatusBasis::ExplicitStudentStatus,
            interval,
            proof: proof(item.source().status_proof(), athlete, query)?,
        });
    }
    Ok(output)
}

pub(super) fn contexts<'a>(
    facts: &'a CheckedDocumentaryParticipationFactsV1<'a>,
    athlete: &'a AthleteId,
    query: CandidateQuery<'a>,
) -> ReportResult<Vec<ContextFact<'a>>> {
    let mut output = reserved(facts.sources().len())?;
    for item in facts.sources() {
        if item.season() == DocumentarySeasonApplicabilityV1::Exact {
            output.push(ContextFact {
                context: ParticipationContext::Scholastic,
                proof: proof(item.source().season_proof(), athlete, query)?,
            });
        }
    }
    Ok(output)
}

pub(super) fn windows<'a>(
    facts: &'a CheckedDocumentaryParticipationFactsV1<'a>,
    intervals: &'a [ProfileDateRangeV1],
    athlete: &'a AthleteId,
    query: CandidateQuery<'a>,
) -> ReportResult<Vec<InSeasonWindowFact<'a>>> {
    let mut output = reserved(facts.sources().len())?;
    for (item, interval) in facts.sources().iter().zip(intervals) {
        if item.season() == DocumentarySeasonApplicabilityV1::Exact {
            output.push(InSeasonWindowFact {
                identity: EligibilityWindowIdentity {
                    school_year: item.source().school_year(),
                    cohort: item.source().cohort(),
                    participation_rule: query.binding.participation_rule,
                },
                interval,
                boundary_proof: proof(item.source().season_proof(), athlete, query)?,
            });
        }
    }
    Ok(output)
}

fn proof<'a>(
    value: &'a IndependentParticipationProofV1,
    athlete: &'a AthleteId,
    query: CandidateQuery<'a>,
) -> ReportResult<ProofReference<'a>> {
    Ok(ProofReference {
        fact: ObjectRef::parse(value.interpretation()).map_err(source)?,
        capture: ObjectRef::parse(value.capture()).map_err(source)?,
        interpretation: ObjectRef::parse(value.interpretation()).map_err(source)?,
        locator: ObjectRef::parse(value.locator()).map_err(source)?,
        athlete,
        known: KnownBounds::new(
            query.binding.frozen.lineage,
            query.binding.frozen.revision,
            query.binding.frozen.revision,
        )
        .map_err(source)?,
        acquired_at: value.acquired_at(),
    })
}

fn reserved<T>(count: usize) -> ReportResult<Vec<T>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| crate::grouping::GroupingError::Allocation {
            stage: "documentary semantic facts",
        })?;
    Ok(output)
}
