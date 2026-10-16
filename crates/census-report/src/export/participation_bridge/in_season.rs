use super::{SourceCohortAgreement, SourceParticipationError};
use census_crawl::milesplit::VerifiedRetainedResultMetadata;
use census_crawl::participation_evidence::{
    retained_result_venue, IndependentParticipationProofV1, SubjectOwnershipV1,
    VerifiedIndependentParticipationV1,
};
use census_domain::affiliation_candidates::{KnownBounds, ObjectRef};
use census_domain::model::{CanonicalPerformance, Gender, ProfileDateRangeV1, Sport};
use census_domain::participation_candidates::{
    CandidateQuery, EligibilityWindowIdentity, HighSchoolStatusFact, InSeasonWindowFact,
    ProofReference, StatusBasis, StudentStatus,
};

pub(super) fn intervals(
    source: &VerifiedRetainedResultMetadata,
    independent: &[VerifiedIndependentParticipationV1],
) -> Result<Vec<ProfileDateRangeV1>, SourceParticipationError> {
    if independent.len() > 16 {
        return Err(SourceParticipationError::ForeignPerformance);
    }
    if independent.is_empty() {
        return Ok(Vec::new());
    }
    let venue =
        retained_result_venue(source).map_err(|_| SourceParticipationError::ForeignPerformance)?;
    independent
        .iter()
        .map(|item| {
            validate(source, item)?;
            if item.venue() != venue {
                return Err(SourceParticipationError::ForeignPerformance);
            }
            ProfileDateRangeV1::new(item.day().clone(), item.day().clone()).map_err(Into::into)
        })
        .collect()
}

fn validate(
    source: &VerifiedRetainedResultMetadata,
    item: &VerifiedIndependentParticipationV1,
) -> Result<(), SourceParticipationError> {
    let SubjectOwnershipV1::ProviderLink(person) = item.ownership() else {
        return Err(SourceParticipationError::ForeignPerformance);
    };
    let row = source.result().row();
    let expected_name = format!("{} {}", row.first_name.trim(), row.last_name.trim());
    if item.store_identity() != source.store_identity()
        || item.snapshot_sequence() != source.snapshot_sequence()
    {
        return Err(SourceParticipationError::ForeignSnapshot);
    }
    if row.gender != Gender::Boys
        || row.grad_year != Some(item.cohort())
        || person.namespace != row.source_athlete.namespace
        || person.id != row.source_athlete.id
        || item.subject().name() != expected_name
        || item.subject().school() != "Detroit Catholic Central"
        || row
            .provider
            .get("teamName")
            .and_then(serde_json::Value::as_str)
            != Some("Detroit Catholic Central")
        || source.page().region.as_deref() != Some("MI")
        || source.page().sport != Some(Sport::OutdoorTrack)
        || source.page().school_year != item.school_year()
        || source.page().meet.name != item.competition()
    {
        return Err(SourceParticipationError::ForeignPerformance);
    }
    Ok(())
}

pub(super) fn status<'a>(
    query: CandidateQuery<'a>,
    performance: &'a CanonicalPerformance,
    sources: &'a [VerifiedIndependentParticipationV1],
    intervals: &'a [ProfileDateRangeV1],
) -> Result<Vec<HighSchoolStatusFact<'a>>, SourceParticipationError> {
    sources
        .iter()
        .zip(intervals)
        .map(|(source, interval)| {
            Ok(HighSchoolStatusFact {
                status: StudentStatus::HighSchool,
                basis: StatusBasis::ExplicitStudentStatus,
                interval,
                proof: proof(query, performance, source.status_proof())?,
            })
        })
        .collect()
}

pub(super) fn windows<'a>(
    query: CandidateQuery<'a>,
    performance: &'a CanonicalPerformance,
    sources: &'a [VerifiedIndependentParticipationV1],
    intervals: &'a [ProfileDateRangeV1],
    cohort: SourceCohortAgreement,
) -> Result<Vec<InSeasonWindowFact<'a>>, SourceParticipationError> {
    let SourceCohortAgreement::Resolved(cohort) = cohort else {
        return Ok(Vec::new());
    };
    sources
        .iter()
        .zip(intervals)
        .map(|(source, interval)| {
            Ok(InSeasonWindowFact {
                identity: EligibilityWindowIdentity {
                    school_year: source.school_year(),
                    cohort,
                    participation_rule: query.binding.participation_rule,
                },
                interval,
                boundary_proof: proof(query, performance, source.season_proof())?,
            })
        })
        .collect()
}

fn proof<'a>(
    query: CandidateQuery<'a>,
    performance: &'a CanonicalPerformance,
    source: &'a IndependentParticipationProofV1,
) -> Result<ProofReference<'a>, SourceParticipationError> {
    let frozen = query.binding.frozen;
    Ok(ProofReference {
        fact: ObjectRef::parse(source.interpretation())?,
        capture: ObjectRef::parse(source.capture())?,
        interpretation: ObjectRef::parse(source.interpretation())?,
        locator: ObjectRef::parse(source.locator())?,
        athlete: &performance.athlete,
        known: KnownBounds::new(frozen.lineage, frozen.revision, frozen.revision)?,
        acquired_at: source.acquired_at(),
    })
}

pub(super) fn references(
    sources: &[VerifiedIndependentParticipationV1],
) -> impl Iterator<Item = String> + '_ {
    sources.iter().flat_map(|source| {
        std::iter::once(source.reference().as_str().to_owned()).chain(
            [source.status_proof(), source.season_proof()]
                .into_iter()
                .flat_map(|proof| {
                    [
                        proof.capture().to_owned(),
                        proof.interpretation().to_owned(),
                        proof.locator().to_owned(),
                    ]
                }),
        )
    })
}
