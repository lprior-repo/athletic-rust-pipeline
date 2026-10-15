use super::PublicIdentityError;
use crate::{Entity, StoreError, StoreResult, StoreSnapshot, Table};
use census_domain::model::{
    separate_generic_identity_cases, AcceptedAthleteIdentity, AthleteCandidateId,
    AthleteIdentityIndex, CanonicalAthlete, IdentityApplication, IdentityProjectionBuilder,
    ReviewCase, ReviewVerdictRecord, SourceIdentity,
};

const BUDGET: crate::read::ReadBudget = crate::read::ReadBudget {
    rows: 8192,
    bytes: 16 * 1024 * 1024,
    record_bytes: 1024 * 1024,
};

pub(super) fn accept(
    snapshot: &StoreSnapshot<'_>,
    candidate: &AthleteCandidateId,
    at: &str,
) -> StoreResult<(AcceptedAthleteIdentity, SourceIdentity)> {
    let mut index = AthleteIdentityIndex::default();
    let mut source = None;
    let mut athletes = collect::<CanonicalAthlete>(snapshot, Table::Athletes)?;
    snapshot.enrich_athletes_bounded(&mut athletes, BUDGET)?;
    for athlete in athletes {
        index.observe(&athlete)?;
        if athlete.id.as_str() == candidate.as_str() {
            source = athlete.source;
        }
    }
    let source = source.ok_or(PublicIdentityError::NotSourceBound)?;
    let cases = collect::<ReviewCase>(snapshot, Table::ReviewCases)?;
    let verdicts = collect::<ReviewVerdictRecord>(snapshot, Table::IdentityVerdicts)?;
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

fn collect<T: Entity>(snapshot: &StoreSnapshot<'_>, table: Table) -> StoreResult<Vec<T>> {
    let mut rows = Vec::new();
    snapshot.for_each_merged_bounded(table, BUDGET, |row, _| {
        rows.try_reserve(1).map_err(|error| StoreError::Refused {
            detail: format!("public identity input allocation: {error}"),
        })?;
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}
