use census_crawl::milesplit::{RetainedPerformanceDate, VerifiedRetainedResultMetadata};
use census_crawl::participation_evidence::VerifiedIndependentParticipationV1;
use census_domain::affiliation_candidates::{KnownBounds, ObjectRef};
use census_domain::model::{serialized_digest, CanonicalPerformance, GradYear, ProfileDateRangeV1};
use census_domain::participation_candidates::{
    assess_participation_for_policy, assess_participation_with_in_season, participation_policy_v4,
    CandidateOutcome, CandidateQuery, ContextFact, HighSchoolStatusFact, InSeasonEvidence,
    ParticipationObservation, ParticipationReasons, PerformanceDateFact, ProofReference,
    ResultOwnership, StatusBasis,
};

mod facts;
mod in_season;
mod validation;

pub(crate) use validation::SourceParticipationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceCohortAgreement {
    Resolved(GradYear),
    Missing,
    Conflicting,
}

#[derive(Debug)]
pub(crate) struct SourceParticipationAssessment {
    outcome: CandidateOutcome,
    reasons: ParticipationReasons,
    evidence_refs: Vec<String>,
}

impl SourceParticipationAssessment {
    pub(crate) fn into_parts(self) -> (CandidateOutcome, ParticipationReasons, Vec<String>) {
        (self.outcome, self.reasons, self.evidence_refs)
    }
}

#[cfg(test)]
pub(crate) fn assess_retained_source_participation(
    query: CandidateQuery<'_>,
    performance: &CanonicalPerformance,
    source: &VerifiedRetainedResultMetadata,
    cohort: SourceCohortAgreement,
) -> Result<SourceParticipationAssessment, SourceParticipationError> {
    assess_with_independent(query, performance, source, cohort, &[])
}

pub(crate) fn assess_with_independent(
    query: CandidateQuery<'_>,
    performance: &CanonicalPerformance,
    source: &VerifiedRetainedResultMetadata,
    cohort: SourceCohortAgreement,
    independent: &[VerifiedIndependentParticipationV1],
) -> Result<SourceParticipationAssessment, SourceParticipationError> {
    validation::validate(query, performance, source)?;
    let fact = serialized_digest(&(
        "milesplit_retained_participation_v1",
        source.result().interpretation_key(),
        source.result().acquisition().reference().manifest_key(),
        source.archive().manifest_key(),
    ))?;
    let proof = proof(query, performance, source, &fact)?;
    let contexts = [ContextFact {
        context: facts::context(source.division()),
        proof,
    }];
    let interval = source_interval(source)?;
    let status = interval
        .as_ref()
        .and_then(|interval| source_status(source, interval, proof));
    let intervals = in_season::intervals(source, independent)?;
    let mut statuses = in_season::status(query, performance, independent, &intervals)?;
    statuses.extend(status);
    let windows = in_season::windows(query, performance, independent, &intervals, cohort)?;
    let observation = observation(
        query,
        performance,
        source,
        cohort,
        CandidateFacts {
            proof,
            contexts: &contexts,
            statuses: &statuses,
        },
    )?;
    let assessment = if query.binding.participation_rule == &participation_policy_v4()? {
        assess_participation_with_in_season(
            query,
            &observation,
            InSeasonEvidence::Uncontested(&windows),
        )?
    } else {
        if !independent.is_empty() {
            return Err(SourceParticipationError::ForeignPerformance);
        }
        assess_participation_for_policy(query, &observation)?
    };
    let mut evidence_refs = evidence_refs(source);
    evidence_refs.extend(in_season::references(independent));
    Ok(SourceParticipationAssessment {
        outcome: assessment.outcome,
        reasons: assessment.reasons,
        evidence_refs,
    })
}

struct CandidateFacts<'a> {
    proof: ProofReference<'a>,
    contexts: &'a [ContextFact<'a>],
    statuses: &'a [HighSchoolStatusFact<'a>],
}

fn observation<'a>(
    query: CandidateQuery<'a>,
    performance: &'a CanonicalPerformance,
    source: &'a VerifiedRetainedResultMetadata,
    cohort: SourceCohortAgreement,
    facts: CandidateFacts<'a>,
) -> Result<ParticipationObservation<'a>, SourceParticipationError> {
    Ok(ParticipationObservation {
        binding: query.binding,
        performance: &performance.id,
        athlete: &performance.athlete,
        ownership: ResultOwnership::Bound {
            performance: &performance.id,
            source_athlete: &source.result().row().source_athlete,
            source_result: ObjectRef::parse(source.result().interpretation_key())?,
            proof: facts.proof,
        },
        date: match source.date() {
            RetainedPerformanceDate::Exact(date) => PerformanceDateFact::Exact(date),
            RetainedPerformanceDate::Unresolved => PerformanceDateFact::Missing,
        },
        cohort: facts::cohort(source.result().row(), cohort, facts.proof),
        contexts: facts.contexts,
        high_school_status: facts.statuses,
        eligibility_windows: &[],
    })
}

fn source_interval(
    source: &VerifiedRetainedResultMetadata,
) -> Result<Option<ProfileDateRangeV1>, SourceParticipationError> {
    match source.date() {
        RetainedPerformanceDate::Exact(date) => {
            Ok(Some(ProfileDateRangeV1::new(date.clone(), date.clone())?))
        }
        RetainedPerformanceDate::Unresolved => Ok(None),
    }
}

fn source_status<'a>(
    source: &VerifiedRetainedResultMetadata,
    interval: &'a ProfileDateRangeV1,
    proof: ProofReference<'a>,
) -> Option<HighSchoolStatusFact<'a>> {
    facts::status(source.division()).map(|status| HighSchoolStatusFact {
        status,
        basis: StatusBasis::ExplicitScholasticResult,
        interval,
        proof,
    })
}

fn evidence_refs(source: &VerifiedRetainedResultMetadata) -> Vec<String> {
    vec![
        source
            .result()
            .acquisition()
            .reference()
            .manifest_key()
            .into(),
        source.result().interpretation_key().into(),
        source.archive().manifest_key().into(),
    ]
}

fn proof<'a>(
    query: CandidateQuery<'a>,
    performance: &'a CanonicalPerformance,
    source: &'a VerifiedRetainedResultMetadata,
    fact: &'a str,
) -> Result<ProofReference<'a>, SourceParticipationError> {
    let frozen = query.binding.frozen;
    Ok(ProofReference {
        fact: ObjectRef::parse(fact)?,
        capture: ObjectRef::parse(source.result().acquisition().reference().manifest_key())?,
        interpretation: ObjectRef::parse(source.result().interpretation_key())?,
        locator: ObjectRef::parse(&source.result().row().locator)?,
        athlete: &performance.athlete,
        known: KnownBounds::new(frozen.lineage, frozen.revision, frozen.revision)?,
        acquired_at: source.acquired_at().max(source.result().acquired_at()),
    })
}

#[cfg(test)]
mod tests;
