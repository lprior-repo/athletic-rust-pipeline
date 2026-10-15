use census_domain::model::{
    separate_generic_identity_cases, AcceptedAthleteIdentity, IdentityApplication, IdentityError,
    IdentityProjectionBuilder, ReviewCase, ReviewState, ReviewVerdictRecord,
};
use census_store::{Store, StoreError, StoreResult, Table, MAX_IDENTITY_APPLICATION_BATCH};
use std::collections::HashMap;

type GenericCases<'a> = HashMap<&'a str, Vec<&'a ReviewCase>>;

pub(super) fn apply_decisions(store: &Store, observed_at: &str) -> StoreResult<usize> {
    let snapshot = store.snapshot();
    let cases: Vec<ReviewCase> = snapshot.scan(Table::ReviewCases)?;
    let verdicts: Vec<ReviewVerdictRecord> = snapshot.scan(Table::IdentityVerdicts)?;
    let (generic, bound) = separate_generic_identity_cases(cases, &verdicts);
    let mut generic_cases: GenericCases<'_> = HashMap::new();
    for case in &generic {
        generic_cases
            .entry(&case.subject_id)
            .or_default()
            .push(case);
    }
    let builder =
        IdentityProjectionBuilder::new(snapshot.athlete_identity_index()?, &bound, &verdicts)?;
    let mut accepted = Vec::with_capacity(MAX_IDENTITY_APPLICATION_BATCH);
    let mut retained = Vec::new();
    let mut applied = 0_usize;
    for application in builder.source_applications(observed_at) {
        stage_application(
            store,
            application?,
            &mut accepted,
            &mut applied,
            &generic_cases,
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
            &generic_cases,
        )?;
    }
    flush_applications(store, &mut accepted, &mut applied, &generic_cases)?;
    store.replace_many(Table::ReviewCases, &retained)?;
    Ok(applied)
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
