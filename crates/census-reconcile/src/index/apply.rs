use census_domain::model::{
    AcceptedAthleteIdentity, IdentityApplication, IdentityError, IdentityProjectionBuilder,
    ReviewCase, ReviewState, ReviewVerdictRecord, ATHLETE_IDENTITY_FAMILY,
};
use census_store::{Store, StoreError, StoreResult, Table, MAX_IDENTITY_APPLICATION_BATCH};
use std::collections::{BTreeSet, HashMap};

type GenericCases<'a> = HashMap<&'a str, Vec<&'a ReviewCase>>;

struct SupersessionProvenance {
    old_id: String,
    new_id: String,
    reason: String,
}

fn find_supersession_candidates(
    cases: &[ReviewCase],
    verdicts: &[ReviewVerdictRecord],
) -> (Vec<SupersessionProvenance>, Vec<String>) {
    let by_id: HashMap<_, _> = cases.iter().map(|case| (&case.id, case)).collect();
    let by_case: HashMap<_, _> = verdicts
        .iter()
        .map(|row| (row.case_id.as_str(), row))
        .collect();
    let mut provenance = Vec::new();
    let mut to_supersede = Vec::new();

    for case in cases.iter().filter(|case| {
        case.state == ReviewState::Resolved
            && case.family == ATHLETE_IDENTITY_FAMILY
            && !case.member_ids.is_empty()
    }) {
        let same_cluster = |candidate: &ReviewCase| {
            candidate.family == ATHLETE_IDENTITY_FAMILY
                && (candidate.subject_id == case.subject_id
                    || case
                        .member_ids
                        .iter()
                        .any(|member| candidate.member_ids.contains(member)))
        };

        for candidate in cases.iter().filter(|candidate| {
            candidate.state == ReviewState::Retained
                && !candidate.member_ids.is_empty()
                && same_cluster(candidate)
                && candidate.id != case.id
        }) {
            if by_id
                .get(&candidate.id)
                .is_some_and(|existing| existing.state == ReviewState::Superseded)
            {
                continue;
            }
            if by_case
                .get(candidate.id.as_str())
                .is_some_and(|row| row.accepted)
            {
                continue;
            }
            to_supersede.push(candidate.id.clone());
            provenance.push(SupersessionProvenance {
                old_id: candidate.id.clone(),
                new_id: case.id.clone(),
                reason: "obsolete machine-generated identity finding replaced by the current-policy resolution for the same cluster".to_string(),
            });
        }
    }

    (provenance, to_supersede)
}

fn persist_supersessions(store: &Store, to_supersede: &[String]) -> StoreResult<()> {
    if to_supersede.is_empty() {
        return Ok(());
    }

    let snapshot = store.snapshot();
    let mut superseded_cases = Vec::with_capacity(to_supersede.len());
    for old_id in to_supersede {
        let current = snapshot
            .get_merged::<ReviewCase>(Table::ReviewCases, old_id)?
            .ok_or_else(|| StoreError::Invariant {
                detail: format!("retained case {} disappeared", old_id),
            })?;
        if current.state != ReviewState::Retained {
            continue;
        }
        let mut superseded = current;
        superseded.state = ReviewState::Superseded;
        superseded_cases.push(superseded);
    }

    if !superseded_cases.is_empty() {
        store.replace_many(Table::ReviewCases, &superseded_cases)?;
    }

    Ok(())
}

fn superseded_obsolete_retained(
    store: &Store,
    cases: &[ReviewCase],
    verdicts: &[ReviewVerdictRecord],
) -> StoreResult<Vec<SupersessionProvenance>> {
    let (provenance, to_supersede) = find_supersession_candidates(cases, verdicts);
    persist_supersessions(store, &to_supersede)?;
    Ok(provenance)
}

fn refresh_after_supersession(store: &Store) -> StoreResult<Vec<ReviewCase>> {
    let refreshed = store.snapshot();
    refreshed.scan(Table::ReviewCases)
}

pub(super) fn apply_decisions(store: &Store, observed_at: &str) -> StoreResult<usize> {
    let snapshot = store.snapshot();
    let cases: Vec<ReviewCase> = snapshot.scan(Table::ReviewCases)?;
    let verdicts: Vec<ReviewVerdictRecord> = snapshot.scan(Table::IdentityVerdicts)?;

    let provenance = superseded_obsolete_retained(store, &cases, &verdicts)?;
    let refreshed_cases = refresh_after_supersession(store)?;

    let (generic, bound) = separate_generic_cases(refreshed_cases, &verdicts);
    let mut generic_cases: GenericCases<'_> = HashMap::new();
    for case in &generic {
        generic_cases
            .entry(&case.subject_id)
            .or_default()
            .push(case);
    }
    let builder = IdentityProjectionBuilder::new(
        store.snapshot().athlete_identity_index()?,
        &bound,
        &verdicts,
    )?;
    let applied = apply_all_applications(store, &builder, &generic_cases, observed_at)?;

    for record in provenance {
        let mut batch = store.write_batch();
        batch.journal_done(
            "identity_retained_supersession_v1",
            &record.old_id,
            &(record.new_id, record.reason),
        )?;
        batch.commit()?;
    }

    Ok(applied)
}
fn apply_all_applications(
    store: &Store,
    builder: &IdentityProjectionBuilder,
    generic_cases: &GenericCases<'_>,
    observed_at: &str,
) -> StoreResult<usize> {
    let mut accepted = Vec::with_capacity(MAX_IDENTITY_APPLICATION_BATCH);
    let mut retained = Vec::new();
    let mut applied = 0_usize;
    for application in builder.source_applications(observed_at) {
        stage_application(
            store,
            application?,
            &mut accepted,
            &mut applied,
            generic_cases,
        )?;
    }
    for (case, application) in builder.reviewed_applications(observed_at) {
        let application = application?;
        if matches!(application, IdentityApplication::Retained(_)) {
            let mut case = case.clone();
            case.state = ReviewState::Retained;
            retained.push(case);
            if retained.len() == MAX_IDENTITY_APPLICATION_BATCH {
                store.replace_many(Table::ReviewCases, &retained)?;
                retained.clear();
            }
        }
        stage_application(
            store,
            application,
            &mut accepted,
            &mut applied,
            generic_cases,
        )?;
    }
    flush_applications(store, &mut accepted, &mut applied, generic_cases)?;
    store.replace_many(Table::ReviewCases, &retained)?;
    Ok(applied)
}


fn separate_generic_cases(
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

fn stage_application(
    store: &Store,
    application: IdentityApplication,
    accepted: &mut Vec<AcceptedAthleteIdentity>,
    applied: &mut usize,
    generic: &GenericCases<'_>,
) -> StoreResult<()> {
    if let IdentityApplication::Accepted(decision) = application {
        accepted.push(decision);
        if accepted.len() == MAX_IDENTITY_APPLICATION_BATCH {
            flush_applications(store, accepted, applied, generic)?;
        }
    }
    Ok(())
}

fn flush_applications(
    store: &Store,
    accepted: &mut Vec<AcceptedAthleteIdentity>,
    applied: &mut usize,
    generic: &GenericCases<'_>,
) -> StoreResult<()> {
    let written = usize::try_from(store.apply_identity_decisions(accepted)?)
        .map_err(|_| IdentityError::CounterOverflow)?;
    *applied = applied
        .checked_add(written)
        .ok_or(StoreError::CounterOverflow)?;
    supersede_generic(store, accepted, generic)?;
    accepted.clear();
    Ok(())
}

fn supersede_generic(
    store: &Store,
    accepted: &[AcceptedAthleteIdentity],
    generic: &GenericCases<'_>,
) -> StoreResult<()> {
    for decision in accepted {
        let record = decision.record();
        let Some(bound_case) = record.case_id.as_deref() else {
            continue;
        };
        for member in &record.members {
            for case in generic.get(member.subject.as_str()).into_iter().flatten() {
                let snapshot = store.snapshot();
                let current = snapshot
                    .get_merged::<ReviewCase>(Table::ReviewCases, &case.id)?
                    .ok_or_else(|| StoreError::Invariant {
                        detail: format!("generic case {} disappeared", case.id),
                    })?;
                if current.state == ReviewState::Superseded {
                    continue;
                }
                if &current != *case {
                    return Err(StoreError::Invariant {
                        detail: format!("generic case {} changed before supersession", case.id),
                    });
                }
                let mut superseded = current;
                superseded.state = ReviewState::Superseded;
                let mut batch = store.write_batch();
                batch.replace_many(Table::ReviewCases, &[superseded])?;
                batch.journal_done(
                    "identity_generic_supersession_v1",
                    &case.id,
                    &(
                        record.id.as_str(),
                        bound_case,
                        "bound identity applied; generic advice superseded",
                    ),
                )?;
                batch.commit_once_at_evidence_generation(
                    &format!("identity-generic-supersession:{}", case.id),
                    &record.id,
                    snapshot.evidence_generation(),
                )?;
            }
        }
    }
    Ok(())
}
