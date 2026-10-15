mod admission;
mod affected;
mod claims;
mod members;
pub(in crate::public_identity) use affected::record_keys;
pub(in crate::public_identity) use claims::membership_inventory;
pub(in crate::public_identity) use members::{member_rows, members, review_case};
#[cfg(test)]
mod tests;

use crate::public_identity::context::BUDGET;
use crate::public_identity::paged_generation::{
    AuthorityTableV2, RawObjectSource, VerifiedRawGraph,
};
use crate::public_identity::PublicIdentityError;
use crate::read::RetainedEnrichment;
use crate::{Entity, StoreError, StoreResult};
use census_domain::model::{
    separate_generic_identity_cases, AcceptedAthleteIdentity, AthleteCandidateId,
    AthleteIdentityIndex, CanonicalAthlete, IdentityApplication, IdentityProjectionBuilder,
    ReviewCase, ReviewVerdictRecord, SourceIdentity, SourceObservation,
};

const ATHLETE_BATCH: usize = 256;

pub(in crate::public_identity) fn accept_source<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    candidate: &AthleteCandidateId,
    at: &str,
) -> StoreResult<(AcceptedAthleteIdentity, SourceIdentity)> {
    admission::preflight(view)?;
    let (index, source) = indexed(view, Some(candidate))?;
    let source = source.ok_or(PublicIdentityError::NotSourceBound)?;
    let cases = collect::<ReviewCase, S>(view, AuthorityTableV2::ReviewCases)?;
    let verdicts = collect::<ReviewVerdictRecord, S>(view, AuthorityTableV2::IdentityVerdicts)?;
    let (_, bound) = separate_generic_identity_cases(cases, &verdicts);
    let builder = IdentityProjectionBuilder::new(index, &bound, &verdicts)?;
    for application in builder.source_applications(at) {
        if let IdentityApplication::Accepted(accepted) = application? {
            if matches!(accepted.record().members.as_slice(), [member] if &member.subject == candidate)
            {
                return Ok((accepted, source));
            }
        }
    }
    Err(PublicIdentityError::NotSourceBound.into())
}

pub(in crate::public_identity) fn accept_reviewed<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    case_id: &str,
    at: &str,
) -> StoreResult<AcceptedAthleteIdentity> {
    admission::preflight(view)?;
    let (index, _) = indexed(view, None)?;
    let cases = collect::<ReviewCase, S>(view, AuthorityTableV2::ReviewCases)?;
    let verdicts = collect::<ReviewVerdictRecord, S>(view, AuthorityTableV2::IdentityVerdicts)?;
    let (_, bound) = separate_generic_identity_cases(cases, &verdicts);
    let builder = IdentityProjectionBuilder::new(index, &bound, &verdicts)?;
    for (case, application) in builder.reviewed_applications(at) {
        if case.id == case_id {
            return match application? {
                IdentityApplication::Accepted(accepted) => Ok(accepted),
                IdentityApplication::Retained(_) => Err(PublicIdentityError::NotReviewed.into()),
            };
        }
    }
    Err(PublicIdentityError::NotReviewed.into())
}

pub(in crate::public_identity) fn identity_material<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    members: &[AthleteCandidateId],
) -> StoreResult<String> {
    admission::preflight(view)?;
    let (index, _) = indexed(view, None)?;
    let evidence = index.case_evidence("local-adjudication-material", "v1", members)?;
    Ok(ReviewCase::pending_with_evidence(
        census_domain::model::ATHLETE_IDENTITY_FAMILY,
        "local-adjudication-material",
        "local-adjudication-material",
        "v1",
        evidence,
    )
    .id)
}

fn indexed<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    candidate: Option<&AthleteCandidateId>,
) -> StoreResult<(AthleteIdentityIndex, Option<SourceIdentity>)> {
    let mut batch = Vec::new();
    batch.try_reserve_exact(ATHLETE_BATCH).map_err(allocation)?;
    let mut index = AthleteIdentityIndex::default();
    let mut source = None;
    let mut enrichment = RetainedEnrichment::new(BUDGET);
    view.for_each_merged::<CanonicalAthlete>(AuthorityTableV2::Athletes, BUDGET, |row, _| {
        if candidate.is_some_and(|candidate| row.id.as_str() == candidate.as_str()) {
            source = row.source.clone();
        }
        batch.push(row);
        if batch.len() == ATHLETE_BATCH {
            observe_batch(view, &mut batch, &mut enrichment, &mut index)?;
        }
        Ok(())
    })?;
    if !batch.is_empty() {
        observe_batch(view, &mut batch, &mut enrichment, &mut index)?;
    }
    Ok((index, source))
}

fn observe_batch<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    batch: &mut Vec<CanonicalAthlete>,
    enrichment: &mut RetainedEnrichment,
    index: &mut AthleteIdentityIndex,
) -> StoreResult<()> {
    enrichment.enrich(batch, |targets| {
        view.for_each_observation::<SourceObservation>(
            AuthorityTableV2::SourceObservations,
            BUDGET,
            |row, _| targets.accept(&row),
        )?;
        Ok(())
    })?;
    batch
        .drain(..)
        .try_for_each(|row| index.observe(&row).map_err(Into::into))
}

fn collect<T: Entity, S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    table: AuthorityTableV2,
) -> StoreResult<Vec<T>> {
    let mut rows = Vec::new();
    view.for_each_merged::<T>(table, BUDGET, |row, _| {
        rows.try_reserve(1).map_err(allocation)?;
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}

fn allocation(error: std::collections::TryReserveError) -> StoreError {
    StoreError::Refused {
        detail: format!("graph authority allocation: {error}"),
    }
}
