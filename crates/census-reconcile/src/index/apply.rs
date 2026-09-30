use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::{
    AcceptedAthleteIdentity, IdentityApplication, IdentityError, IdentityProjectionBuilder, Store,
    StoreError, StoreResult, Table, MAX_IDENTITY_APPLICATION_BATCH,
};

pub(super) fn apply_decisions(store: &Store, observed_at: &str) -> StoreResult<usize> {
    let snapshot = store.snapshot();
    let cases: Vec<ReviewCase> = snapshot.scan(Table::ReviewCases)?;
    let verdicts: Vec<ReviewVerdictRecord> = snapshot.scan(Table::IdentityVerdicts)?;
    let builder =
        IdentityProjectionBuilder::new(snapshot.athlete_identity_index()?, &cases, &verdicts)?;
    let mut accepted = Vec::with_capacity(MAX_IDENTITY_APPLICATION_BATCH);
    let mut retained = Vec::new();
    let mut applied = 0_usize;
    for application in builder.source_applications(observed_at) {
        stage_application(store, application?, &mut accepted, &mut applied)?;
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
        stage_application(store, application, &mut accepted, &mut applied)?;
    }
    flush_applications(store, &mut accepted, &mut applied)?;
    store.replace_many(Table::ReviewCases, &retained)?;
    Ok(applied)
}

fn stage_application(
    store: &Store,
    application: IdentityApplication,
    accepted: &mut Vec<AcceptedAthleteIdentity>,
    applied: &mut usize,
) -> StoreResult<()> {
    if let IdentityApplication::Accepted(decision) = application {
        accepted.push(decision);
        if accepted.len() == MAX_IDENTITY_APPLICATION_BATCH {
            flush_applications(store, accepted, applied)?;
        }
    }
    Ok(())
}

fn flush_applications(
    store: &Store,
    accepted: &mut Vec<AcceptedAthleteIdentity>,
    applied: &mut usize,
) -> StoreResult<()> {
    let written = usize::try_from(store.apply_identity_decisions(accepted)?)
        .map_err(|_| IdentityError::CounterOverflow)?;
    *applied = applied
        .checked_add(written)
        .ok_or(StoreError::CounterOverflow)?;
    accepted.clear();
    Ok(())
}
