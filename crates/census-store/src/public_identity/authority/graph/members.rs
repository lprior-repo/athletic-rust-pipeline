use super::*;
use census_domain::public_identity_history::ReplayLimits;

pub(in crate::public_identity) fn member_rows<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    members: &[AthleteCandidateId],
) -> StoreResult<Vec<(usize, CanonicalAthlete)>> {
    if members.len() > ReplayLimits::SMALL.members_per_operation {
        return Err(PublicIdentityError::Corrupt("manual member bound").into());
    }
    admission::preflight(view)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(members.len()).map_err(allocation)?;
    let mut ordinal = 0_usize;
    view.for_each_merged::<CanonicalAthlete>(AuthorityTableV2::Athletes, BUDGET, |row, _| {
        let current = ordinal;
        ordinal = ordinal.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        if members
            .iter()
            .any(|member| member.as_str() == row.id.as_str())
        {
            if rows.len() >= members.len() {
                return Err(PublicIdentityError::Corrupt("duplicate manual member").into());
            }
            rows.push((current, row));
        }
        Ok(())
    })?;
    if members.iter().any(|member| {
        !rows
            .iter()
            .any(|(_, row)| row.id.as_str() == member.as_str())
    }) {
        return Err(PublicIdentityError::Corrupt("manual member missing").into());
    }
    Ok(rows)
}

pub(in crate::public_identity) fn review_case<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    case_id: &str,
) -> StoreResult<ReviewCase> {
    admission::preflight(view)?;
    let mut found = None;
    view.for_each_merged::<ReviewCase>(AuthorityTableV2::ReviewCases, BUDGET, |row, _| {
        if row.id == case_id {
            found = Some(row);
        }
        Ok(())
    })?;
    found.ok_or_else(|| PublicIdentityError::Corrupt("manual review case missing").into())
}

pub(in crate::public_identity) fn members<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    projection: &census_domain::public_identity_history::HistoryProjection,
    canonical: &census_domain::public_identity_history::PublicAthleteId,
) -> StoreResult<Vec<AthleteCandidateId>> {
    admission::preflight(view)?;
    let mut members =
        crate::public_identity::local_adjudication::members::Members::new(projection, canonical);
    view.for_each_merged::<CanonicalAthlete>(AuthorityTableV2::Athletes, BUDGET, |row, _| {
        members.observe(&row)
    })?;
    Ok(members.finish())
}
