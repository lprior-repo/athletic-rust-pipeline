use census_crawl::milesplit::{RetainedPerformanceDate, VerifiedRetainedResultMetadata};
use census_domain::model::CanonicalPerformance;
use census_domain::participation_candidates::CandidateQuery;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SourceParticipationError {
    #[error("retained participation source belongs to another store or snapshot")]
    ForeignSnapshot,
    #[error("canonical performance differs from retained source result ownership, mark or date")]
    ForeignPerformance,
    #[error(transparent)]
    Assessment(#[from] census_domain::participation_candidates::ParticipationInputError),
    #[error(transparent)]
    Evidence(#[from] census_domain::affiliation_candidates::AffiliationCandidateError),
    #[error(transparent)]
    Profile(#[from] census_domain::model::ProfileError),
    #[error(transparent)]
    Canonical(#[from] census_domain::model::CanonicalJsonError),
}

pub(super) fn validate(
    query: CandidateQuery<'_>,
    performance: &CanonicalPerformance,
    source: &VerifiedRetainedResultMetadata,
) -> Result<(), SourceParticipationError> {
    let frozen = query.binding.frozen;
    if frozen.lineage.as_str() != source.store_identity().as_str()
        || frozen.revision.get() != source.snapshot_sequence()
    {
        return Err(SourceParticipationError::ForeignSnapshot);
    }
    let row = source.result().row();
    if performance.source_key != format!("milesplit_result:{}", row.result_id)
        || performance.source_athlete.as_ref().is_none_or(|owner| {
            owner.namespace != row.source_athlete.namespace || owner.id != row.source_athlete.id
        })
        || performance.mark != row.mark
        || performance.timing != row.timing
        || matches!(source.date(), RetainedPerformanceDate::Exact(date) if date.as_str() != performance.date)
    {
        return Err(SourceParticipationError::ForeignPerformance);
    }
    Ok(())
}
