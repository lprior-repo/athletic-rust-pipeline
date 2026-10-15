use census_crawl::milesplit::{RetainedPerformanceDate, VerifiedRetainedResultMetadata};
use census_domain::affiliation_candidates::{KnownBounds, ObjectRef};
use census_domain::model::{serialized_digest, CanonicalPerformance, GradYear, ProfileDateRangeV1};
use census_domain::participation_candidates::{
    assess_participation_for_policy, CandidateOutcome, CandidateQuery, ContextFact,
    HighSchoolStatusFact, ParticipationObservation, ParticipationReasons, PerformanceDateFact,
    ProofReference, ResultOwnership, StatusBasis,
};

mod facts;
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

pub(crate) fn assess_retained_source_participation(
    query: CandidateQuery<'_>,
    performance: &CanonicalPerformance,
    source: &VerifiedRetainedResultMetadata,
    cohort: SourceCohortAgreement,
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
    let interval = match source.date() {
        RetainedPerformanceDate::Exact(date) => {
            Some(ProfileDateRangeV1::new(date.clone(), date.clone())?)
        }
        RetainedPerformanceDate::Unresolved => None,
    };
    let status = interval.as_ref().and_then(|interval| {
        facts::status(source.division()).map(|status| HighSchoolStatusFact {
            status,
            basis: StatusBasis::ExplicitScholasticResult,
            interval,
            proof,
        })
    });
    let observation = ParticipationObservation {
        binding: query.binding,
        performance: &performance.id,
        athlete: &performance.athlete,
        ownership: ResultOwnership::Bound {
            performance: &performance.id,
            source_athlete: &source.result().row().source_athlete,
            source_result: ObjectRef::parse(source.result().interpretation_key())?,
            proof,
        },
        date: match source.date() {
            RetainedPerformanceDate::Exact(date) => PerformanceDateFact::Exact(date),
            RetainedPerformanceDate::Unresolved => PerformanceDateFact::Missing,
        },
        cohort: facts::cohort(source.result().row(), cohort, proof),
        contexts: &contexts,
        high_school_status: status.as_slice(),
        eligibility_windows: &[],
    };
    let assessment = assess_participation_for_policy(query, &observation)?;
    Ok(SourceParticipationAssessment {
        outcome: assessment.outcome,
        reasons: assessment.reasons,
        evidence_refs: evidence_refs(source),
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
