use std::error::Error;

use super::{key, ledger};
use crate::restate_services::wire::{
    JurisdictionRequest, StageOutcome, TeamsAttemptProgress, TeamsSourceOutcome, TeamsSourceRequest,
};
use crate::restate_services::JobError;
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_reconcile::identity::Revision;
use census_store::Store;

type TestResult = Result<(), Box<dyn Error>>;

mod registration;
mod registration_cancellation;
mod settlement;

fn request() -> Result<TeamsSourceRequest, Box<dyn Error>> {
    Ok(TeamsSourceRequest {
        jurisdiction: JurisdictionRequest {
            jurisdiction: UsJurisdiction::Ohio,
            season: SchoolYear::new(2026).ok_or("invalid school year")?,
            revision: Revision(1),
            refresh: false,
            limit_per_state: None,
            concurrency: 1,
            observed_on: None,
            authorized_hosts: Vec::new(),
            source_parallelism: 1,
        },
        source: "arbiter_orgs".to_string(),
        observed_on: "2026-10-02".to_string(),
    })
}

fn reserve(store: &Store, request: &TeamsSourceRequest) -> Result<(u8, String), Box<dyn Error>> {
    let registered = ledger::register(store, &key(request), request)?;
    match ledger::begin(store, &key(request), request, &registered)? {
        ledger::Admission::Reserved {
            attempt,
            observed_on,
        } => Ok((attempt, observed_on)),
        ledger::Admission::Settled(_) => Err("source unexpectedly settled before admission".into()),
    }
}

#[test]
fn transient_admissions_survive_reopen_and_exhaust_without_attempt_four() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    for expected in 1..=3 {
        let store = Store::open(root.path())?;
        let (attempt, _) = reserve(&store, &request)?;
        check!(eq; attempt, expected);
        let finished = ledger::finish(
            &store,
            &key(&request),
            attempt,
            (Err(JobError::Transient {
                message: format!("real source failure {attempt}"),
            }))
            .into(),
        )?;
        if expected < 3 {
            check!(matches!(
                finished,
                ledger::Completion::Retry(JobError::Transient { .. })
            ));
        } else {
            check!(matches!(
                finished,
                ledger::Completion::Settled(TeamsSourceOutcome::Exhausted { attempts: 3, .. })
            ));
        }
    }
    let store = Store::open(root.path())?;
    check!(matches!(
        ledger::begin(
            &store,
            &key(&request),
            &request,
            &ledger::register(&store, &key(&request), &request)?
        )?,
        ledger::Admission::Settled(TeamsSourceOutcome::Exhausted { attempts: 3, .. })
    ));
    check!(!store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/4/reserved", key(&request))
    )?);
    Ok(())
}

#[test]
fn acknowledged_completion_replays_original_progress_and_date_after_midnight() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut request = request()?;
    let store = Store::open(root.path())?;
    let (attempt, observed_on) = reserve(&store, &request)?;
    let outcome = StageOutcome {
        records: 37,
        at: observed_on,
        errors: Vec::new(),
        notes: vec!["retained independently acquired coaches".to_string()],
    };
    let expected = serde_json::to_value(&outcome)?;
    ledger::finish(&store, &key(&request), attempt, (Ok(outcome)).into())?;
    store.flush()?;
    drop(store);
    request.observed_on = "2026-10-03".to_string();
    request.jurisdiction.source_parallelism = 8;
    let store = Store::open(root.path())?;
    let ledger::Admission::Settled(TeamsSourceOutcome::Completed { outcome, .. }) = ledger::begin(
        &store,
        &key(&request),
        &request,
        &ledger::register(&store, &key(&request), &request)?,
    )?
    else {
        return Err("completed source became physically runnable".into());
    };
    check!(eq; serde_json::to_value(outcome)?, expected);
    check!(!store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/2/reserved", key(&request))
    )?);
    Ok(())
}

#[test]
fn reserved_but_unacknowledged_attempts_are_consumed_and_never_claim_exhaustion() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    for expected in 1..=3 {
        let store = Store::open(root.path())?;
        check!(eq; reserve(&store, &request)?.0, expected);
    }
    let store = Store::open(root.path())?;
    check!(matches!(
        ledger::begin(
            &store,
            &key(&request),
            &request,
            &ledger::register(&store, &key(&request), &request)?
        )?,
        ledger::Admission::Settled(TeamsSourceOutcome::Interrupted {
            attempts: Some(3),
            ..
        })
    ));
    check!(!store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/4/reserved", key(&request))
    )?);
    Ok(())
}

#[test]
fn permanent_failure_stops_first_attempt_and_retains_its_reason() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let store = Store::open(root.path())?;
    let (attempt, _) = reserve(&store, &request)?;
    ledger::finish(
        &store,
        &key(&request),
        attempt,
        (Err(JobError::Terminal {
            message: "public source access refused".to_string(),
        }))
        .into(),
    )?;
    drop(store);
    let store = Store::open(root.path())?;
    let ledger::Admission::Settled(TeamsSourceOutcome::Terminal { message, .. }) = ledger::begin(
        &store,
        &key(&request),
        &request,
        &ledger::register(&store, &key(&request), &request)?,
    )?
    else {
        return Err("permanent failure became physically runnable".into());
    };
    check!(eq; message, "public source access refused");
    check!(!store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/2/reserved", key(&request))
    )?);
    Ok(())
}

#[test]
fn changed_source_identity_cannot_reuse_existing_attempt_authority() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut request = request()?;
    let operation = key(&request);
    let store = Store::open(root.path())?;
    reserve(&store, &request)?;
    let registered = ledger::register(&store, &operation, &request)?;
    request.source = "coach_directories".to_string();
    check!(matches!(
        ledger::begin(&store, &operation, &request, &registered),
        Err(JobError::Terminal { .. })
    ));
    Ok(())
}

#[test]
fn mixed_unknown_and_transient_attempts_never_claim_three_acknowledged_failures() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let store = Store::open(root.path())?;
    reserve(&store, &request)?;
    for expected in 2..=3 {
        let (attempt, _) = reserve(&store, &request)?;
        check!(eq; attempt, expected);
        ledger::finish(
            &store,
            &key(&request),
            attempt,
            (Err(JobError::Transient {
                message: format!("acknowledged failure {attempt}"),
            }))
            .into(),
        )?;
    }
    let ledger::Admission::Settled(TeamsSourceOutcome::Interrupted {
        attempts, progress, ..
    }) = ledger::begin(
        &store,
        &key(&request),
        &request,
        &ledger::register(&store, &key(&request), &request)?,
    )?
    else {
        return Err("unknown physical outcome was misrepresented as exhausted failures".into());
    };
    check!(eq; attempts, Some(3));
    check!(matches!(
        progress.first(),
        Some(TeamsAttemptProgress::Unknown { attempt: 1 })
    ));
    check!(matches!(
        progress.last(),
        Some(TeamsAttemptProgress::Transient { attempt: 3, .. })
    ));
    Ok(())
}
