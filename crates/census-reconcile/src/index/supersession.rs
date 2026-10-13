use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use census_domain::model::{
    AppliedAthleteIdentity, AthleteCandidateId, AthleteIdentityIndex, IdentityApplication,
    IdentityProjectionBuilder, IdentityStatus, ReviewCase, ReviewState, ReviewVerdictRecord,
    ATHLETE_IDENTITY_FAMILY,
};
use census_store::{DerivedStage, StoreResult, StoreSnapshot, Table};

const SUPERSESSION_REASON: &str = "obsolete machine-generated identity finding replaced by the current-policy resolution for the same cluster";

#[derive(Debug)]
pub(super) struct SupersessionRecord {
    pub(super) old_id: String,
    pub(super) new_id: String,
    pub(super) reason: String,
}

pub(super) fn stage_findings(
    stage: &mut DerivedStage<'_>,
    closed: &[ReviewCase],
    current: &[ReviewCase],
    provenance: Vec<SupersessionRecord>,
) -> StoreResult<()> {
    stage.replace_many(Table::ReviewCases, closed)?;
    stage.replace_many(Table::ReviewCases, current)?;
    for record in provenance {
        stage.journal_done_once(
            "identity_retained_supersession_v1",
            &record.old_id,
            &(record.new_id, record.reason),
        )?;
    }
    Ok(())
}

pub(super) fn plan(
    snapshot: &StoreSnapshot<'_>,
    stored: &HashMap<String, ReviewCase>,
    current: &[ReviewCase],
    observed_at: &str,
) -> StoreResult<(Vec<ReviewCase>, Vec<SupersessionRecord>)> {
    let live: HashSet<&str> = current.iter().map(|case| case.id.as_str()).collect();
    let cases: Vec<ReviewCase> = stored.values().cloned().collect();
    let verdicts: Vec<ReviewVerdictRecord> = snapshot.scan(Table::IdentityVerdicts)?;
    let index = snapshot.athlete_identity_index()?;
    let obsolete: Vec<&ReviewCase> = cases
        .iter()
        .filter(|case| {
            case.state == ReviewState::Retained
                && case.family == ATHLETE_IDENTITY_FAMILY
                && !case.member_ids.is_empty()
                && !live.contains(case.id.as_str())
                && !verdicts.iter().any(|row| row.case_id == case.id)
        })
        .filter_map(|case| match obsolete_case(&index, case) {
            Ok(true) => Some(Ok(case)),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<StoreResult<_>>()?;
    let proven = proven_successors(snapshot, &cases, &verdicts, index, observed_at)?;
    let mut closed: Vec<ReviewCase> = cases
        .iter()
        .filter(|case| case.state == ReviewState::Pending && !live.contains(case.id.as_str()))
        .cloned()
        .map(|mut case| {
            case.state = ReviewState::Superseded;
            case
        })
        .collect();
    let mut provenance = Vec::new();
    for old in obsolete {
        let mut successors = cases
            .iter()
            .filter(|case| proven.contains(case.id.as_str()) && same_members(old, case));
        let Some(successor) = successors.next() else {
            continue;
        };
        if successors.next().is_some() {
            continue;
        }
        let mut closed_case = old.clone();
        closed_case.state = ReviewState::Superseded;
        closed.push(closed_case);
        provenance.push(SupersessionRecord {
            old_id: old.id.clone(),
            new_id: successor.id.clone(),
            reason: SUPERSESSION_REASON.to_owned(),
        });
    }
    closed.sort_by(|left, right| left.id.cmp(&right.id));
    provenance.sort_by(|left, right| left.old_id.cmp(&right.old_id));
    Ok((closed, provenance))
}

fn obsolete_case(index: &AthleteIdentityIndex, case: &ReviewCase) -> StoreResult<bool> {
    let members: BTreeSet<_> = case.member_ids.iter().collect();
    if members.len() != case.member_ids.len()
        || !members.iter().any(|id| id.as_str() == case.subject_id)
        || members.iter().any(|id| index.member(id).is_none())
    {
        return Ok(false);
    }
    let evidence = index.case_evidence(&case.subject, &case.detail, &case.member_ids)?;
    Ok(!case.matches_evidence(&evidence))
}

fn same_members(left: &ReviewCase, right: &ReviewCase) -> bool {
    let left_ids: BTreeSet<&AthleteCandidateId> = left.member_ids.iter().collect();
    let right_ids: BTreeSet<&AthleteCandidateId> = right.member_ids.iter().collect();
    !left_ids.is_empty()
        && left_ids.len() == left.member_ids.len()
        && right_ids.len() == right.member_ids.len()
        && left_ids == right_ids
}

fn proven_successors(
    snapshot: &StoreSnapshot<'_>,
    cases: &[ReviewCase],
    verdicts: &[ReviewVerdictRecord],
    index: AthleteIdentityIndex,
    observed_at: &str,
) -> StoreResult<BTreeSet<String>> {
    let decisions: Vec<AppliedAthleteIdentity> = snapshot.scan(Table::AthleteIdentityDecisions)?;
    let mut builder = IdentityProjectionBuilder::new(index, cases, verdicts)?;
    let mut expected = BTreeMap::new();
    for (case, application) in builder.reviewed_applications(observed_at) {
        if let IdentityApplication::Accepted(decision) = application? {
            expected.insert(case.id.clone(), decision.record().id.clone());
        }
    }
    let mut accepted_persisted = BTreeSet::new();
    for decision in &decisions {
        if builder.consider(decision)?.is_none() {
            accepted_persisted.insert(decision.id.as_str());
        }
    }
    let mut proven: BTreeSet<String> = expected
        .into_iter()
        .filter_map(|(case_id, application_id)| {
            accepted_persisted
                .contains(application_id.as_str())
                .then_some(case_id)
        })
        .collect();
    let projection = builder.finish()?;
    for case in cases {
        if !proven.contains(&case.id) {
            continue;
        }
        if case
            .member_ids
            .iter()
            .map(|id| projection.status(id.as_str()))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|status| *status != IdentityStatus::Verified)
        {
            proven.remove(&case.id);
        }
    }
    Ok(proven)
}

pub(super) fn publication_digest(
    snapshot: &StoreSnapshot<'_>,
    stored: &HashMap<String, ReviewCase>,
    base: &str,
    planned: &[SupersessionRecord],
    school_year: census_domain::model::SchoolYear,
) -> StoreResult<String> {
    let mut witnesses = BTreeMap::new();
    for old in stored.values().filter(|case| {
        case.state == ReviewState::Superseded
            && case.family == ATHLETE_IDENTITY_FAMILY
            && !case.member_ids.is_empty()
    }) {
        let Some(payload) =
            snapshot.journal_payload("identity_retained_supersession_v1", &old.id)?
        else {
            continue;
        };
        let (new_id, reason): (String, String) =
            serde_json::from_value(payload).map_err(|source| census_store::StoreError::Json {
                detail: "decoding completed identity supersession witness".to_owned(),
                source,
            })?;
        let successor = stored.get(&new_id).filter(|case| {
            case.id != old.id && case.family == ATHLETE_IDENTITY_FAMILY && same_members(old, case)
        });
        if successor.is_none() || reason != SUPERSESSION_REASON {
            return Err(census_store::StoreError::PublicationRefused {
                detail: format!(
                    "completed supersession journal for {} does not bind its historical cluster",
                    old.id
                ),
            });
        }
        witnesses.insert(old.id.clone(), witness(old, new_id, reason));
    }
    for record in planned {
        let old =
            stored
                .get(&record.old_id)
                .ok_or_else(|| census_store::StoreError::Invariant {
                    detail: "planned identity supersession lost its original finding".to_owned(),
                })?;
        witnesses.insert(
            record.old_id.clone(),
            witness(old, record.new_id.clone(), record.reason.clone()),
        );
    }
    let applied = snapshot.tables_digest(&[Table::AthleteIdentityDecisions])?;
    census_domain::model::serialized_digest(&(
        base,
        "index-supersession-policy:1",
        school_year,
        applied,
        witnesses,
    ))
    .map_err(census_domain::model::IdentityError::from)
    .map_err(census_store::StoreError::from)
}

fn witness(old: &ReviewCase, new_id: String, reason: String) -> (ReviewCase, String, String) {
    let mut original = old.clone();
    original.state = ReviewState::Retained;
    original.member_ids.sort();
    (original, new_id, reason)
}

#[cfg(test)]
mod tests;
