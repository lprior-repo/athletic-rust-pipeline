use super::{ReviewCase, ReviewState, ReviewVerdictRecord, ATHLETE_IDENTITY_FAMILY};
use std::collections::{BTreeSet, HashMap};

pub fn separate_generic_identity_cases(
    cases: Vec<ReviewCase>,
    verdicts: &[ReviewVerdictRecord],
) -> (Vec<ReviewCase>, Vec<ReviewCase>) {
    let by_case: HashMap<_, _> = verdicts
        .iter()
        .map(|row| (row.case_id.as_str(), row))
        .collect();
    let members: BTreeSet<_> = cases
        .iter()
        .filter(|case| {
            case.family == ATHLETE_IDENTITY_FAMILY && case.state == ReviewState::Resolved
        })
        .flat_map(|case| case.member_ids.iter().cloned())
        .collect();
    cases.into_iter().partition(|case| {
        case.family == ATHLETE_IDENTITY_FAMILY
            && case.member_ids.is_empty()
            && members.contains(case.subject_id.as_str())
            && matches!(case.state, ReviewState::Pending | ReviewState::Retained)
            && by_case
                .get(case.id.as_str())
                .is_none_or(|row| !row.accepted && row.kind == "insufficient_evidence")
    })
}

#[cfg(test)]
mod tests;
