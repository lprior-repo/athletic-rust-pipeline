mod admission;
mod affected;
mod claims;
mod members;
pub(in crate::public_identity) use affected::record_keys;
pub(in crate::public_identity) use claims::membership_inventory;
pub(in crate::public_identity) use members::{member_rows, members, review_case};
#[cfg(test)]
mod tests;

const BUDGET: crate::read::ReadBudget = crate::read::ReadBudget {
    rows: 1_048_576,
    bytes: 512 * 1024 * 1024,
    record_bytes: 1024 * 1024,
};
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
use std::collections::BTreeMap;

pub(in crate::public_identity) type AcceptedSources =
    BTreeMap<AthleteCandidateId, (AcceptedAthleteIdentity, SourceIdentity)>;
type SourceOwners = BTreeMap<AthleteCandidateId, SourceIdentity>;

pub(in crate::public_identity) fn accept_source<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    candidate: &AthleteCandidateId,
    at: &str,
) -> StoreResult<(AcceptedAthleteIdentity, SourceIdentity)> {
    accepted_sources(view, at)?
        .remove(candidate)
        .ok_or_else(|| PublicIdentityError::NotSourceBound.into())
}

pub(in crate::public_identity) fn accepted_sources<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    at: &str,
) -> StoreResult<AcceptedSources> {
    admission::preflight(view)?;
    let (index, mut sources) = indexed(view)?;
    let cases = collect::<ReviewCase, S>(view, AuthorityTableV2::ReviewCases)?;
    let verdicts = collect::<ReviewVerdictRecord, S>(view, AuthorityTableV2::IdentityVerdicts)?;
    let (_, bound) = separate_generic_identity_cases(cases, &verdicts);
    let builder = IdentityProjectionBuilder::new(index, &bound, &verdicts)?;
    let mut accepted_sources = BTreeMap::new();
    for application in builder.source_applications(at) {
        if let IdentityApplication::Accepted(accepted) = application? {
            let [member] = accepted.record().members.as_slice() else {
                continue;
            };
            let candidate = member.subject.clone();
            let Some(source) = sources.remove(&candidate) else {
                continue;
            };
            if accepted_sources
                .insert(candidate, (accepted, source))
                .is_some()
            {
                return Err(PublicIdentityError::Corrupt("duplicate accepted source").into());
            }
        }
    }
    Ok(accepted_sources)
}

pub(in crate::public_identity) fn accept_reviewed<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    case_id: &str,
    at: &str,
) -> StoreResult<AcceptedAthleteIdentity> {
    admission::preflight(view)?;
    let (index, _) = indexed(view)?;
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
    let (index, _) = indexed(view)?;
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
) -> StoreResult<(AthleteIdentityIndex, SourceOwners)> {
    let mut athletes = collect::<CanonicalAthlete, S>(view, AuthorityTableV2::Athletes)?;
    let mut enrichment = RetainedEnrichment::new(BUDGET);
    enrichment.enrich(&mut athletes, |targets| {
        view.for_each_observation::<SourceObservation>(
            AuthorityTableV2::SourceObservations,
            BUDGET,
            |row, _| targets.accept(&row),
        )?;
        Ok(())
    })?;
    let mut index = AthleteIdentityIndex::default();
    let mut sources = BTreeMap::new();
    for athlete in athletes {
        let candidate = athlete.id.cast();
        index.observe(&athlete)?;
        if let Some(source) = athlete.source {
            if sources.insert(candidate, source).is_some() {
                return Err(PublicIdentityError::Corrupt("duplicate source owner").into());
            }
        }
    }
    Ok((index, sources))
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
