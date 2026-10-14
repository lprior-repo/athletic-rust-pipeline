use super::{
    AffiliationCandidateError, CandidateQuery, CandidateSelection, DeclaredFrozenView,
    FactDisposition, LocallyCheckedAssertion, LocallyCheckedChange, NonSchoolKind, SelectionGrant,
};
use crate::model::{AthleteId, CanonicalJsonError, ProfileId, QueryProfileV1, SchoolId, Sport};
use chrono::{DateTime, Utc};

mod projection;
mod request;

#[derive(Debug, Clone, Copy)]
pub struct CurrentAffiliationRequest<'a> {
    pub athlete: &'a AthleteId,
    pub profile: &'a QueryProfileV1,
    pub sport: Sport,
    pub known: DeclaredFrozenView<'a>,
    pub acquisition_cutoff: DateTime<Utc>,
    pub grant: SelectionGrant,
}

#[derive(Debug, Clone, Copy)]
pub struct CurrentAffiliationInventory<'a> {
    pub declared_view: DeclaredFrozenView<'a>,
    pub assertions: &'a [LocallyCheckedAssertion<'a>],
    pub changes: &'a [LocallyCheckedChange<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentAffiliationOutcome<'a> {
    ResolvedSchoolCandidate(&'a SchoolId),
    Conflicted,
    Unknown,
    SubjectUnresolved,
    EndedWithoutReplacement,
    CoopCampusUnresolved,
    NonSchool(NonSchoolKind),
}

#[derive(Debug)]
#[must_use]
pub struct CurrentAffiliationAssessment<'a> {
    pub request: CurrentAffiliationRequest<'a>,
    pub profile_id: ProfileId,
    pub outcome: CurrentAffiliationOutcome<'a>,
    pub facts: Vec<FactDisposition<'a>>,
}

#[derive(Debug, thiserror::Error)]
pub enum CurrentAffiliationError {
    #[error("current affiliation candidate validation failed: {0}")]
    Candidate(#[from] AffiliationCandidateError),
    #[error("current affiliation profile identity failed: {0}")]
    ProfileIdentity(#[from] CanonicalJsonError),
    #[error("current affiliation inventory does not match the exact declared frozen view")]
    InventoryViewMismatch,
    #[error("current affiliation candidate assessment does not support provisional inclusion")]
    ProvisionalInclusionUnsupported,
    #[error("current affiliation sport is not selected by the query profile")]
    SportNotSelected,
    #[error("current affiliation acquisition cutoff follows the declared frozen instant")]
    AcquisitionCutoffAfterFrozenInstant,
    #[error("current affiliation candidate reduction has no consistent school target")]
    InconsistentCandidateReduction,
}

pub fn assess_current_affiliation<'a>(
    request: CurrentAffiliationRequest<'a>,
    inventory: CurrentAffiliationInventory<'a>,
) -> Result<CurrentAffiliationAssessment<'a>, CurrentAffiliationError> {
    request::check(&request, &inventory)?;
    let query = CandidateQuery {
        athlete: request.athlete,
        as_of: &request.profile.input().as_of,
        sport: request.sport,
        known: request.known,
        acquisition_cutoff: request.acquisition_cutoff,
        rule_claim: &request.profile.input().policies.affiliation,
        grant: request.grant,
    };
    let selection = super::selection::select_with_envelope(
        query,
        inventory.assertions,
        inventory.changes,
        super::selection::TemporalEnvelope::IndependentEffectiveAndKnownTime,
    )?;
    Ok(CurrentAffiliationAssessment {
        request,
        profile_id: request.profile.id()?,
        outcome: projection::outcome(&selection)?,
        facts: selection.facts,
    })
}

#[cfg(test)]
mod tests;
