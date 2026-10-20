use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use census_store::{clock::SystemClock, Store};

use super::super::{key, tests::request, TeamsSource};
use super::{baseline, begin_at, finish, register, AdmissionReplay, Attempt, Completion, PHASE};
use crate::restate_services::jurisdiction::JurisdictionCensus;
use crate::restate_services::wire::{StageOutcome, TeamsAttemptProgress, TeamsSourceOutcome};
use crate::restate_services::{CaptureExecution, JobError, Jobs};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn retry(admission: AdmissionReplay) -> TestResult<String> {
    match admission {
        AdmissionReplay::ReplayRetry { message } => Ok(message),
        AdmissionReplay::Reserved { attempt, .. } => {
            Err(format!("replay incorrectly reserved attempt {attempt}").into())
        }
        AdmissionReplay::Settled(outcome) => {
            Err(format!("replay unexpectedly settled: {outcome:?}").into())
        }
    }
}

fn settled(admission: AdmissionReplay) -> TestResult<TeamsSourceOutcome> {
    match admission {
        AdmissionReplay::Settled(outcome) => Ok(outcome),
        AdmissionReplay::Reserved { attempt, .. } => {
            Err(format!("settled replay reserved attempt {attempt}").into())
        }
        AdmissionReplay::ReplayRetry { message } => {
            Err(format!("settled replay incorrectly retried: {message}").into())
        }
    }
}

fn reservation(admission: AdmissionReplay) -> TestResult<(u8, String)> {
    match admission {
        AdmissionReplay::Reserved {
            attempt,
            observed_on,
        } => Ok((attempt, observed_on)),
        AdmissionReplay::ReplayRetry { message } => {
            Err(format!("fresh admission incorrectly retried: {message}").into())
        }
        AdmissionReplay::Settled(outcome) => {
            Err(format!("fresh admission unexpectedly settled: {outcome:?}").into())
        }
    }
}

#[test]
fn baseline_is_read_only_and_requires_registration() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let operation = key(&request);
    check!(matches!(
        baseline(&store, &operation),
        Err(JobError::Terminal { message })
            if message == "teams source baseline has no registered identity"
    ));
    check!(eq; store.journal_keys(PHASE)?, std::collections::HashSet::<String>::new());
    register(&store, &operation, &request)?;
    let before = store.journal_keys(PHASE)?;
    check!(eq; baseline(&store, &operation)?, 0);
    check!(eq; store.journal_keys(PHASE)?, before);
    Ok(())
}

#[test]
fn postcommit_retry_replays_the_same_slot_then_a_new_baseline_advances() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let operation = key(&request);
    let registered = register(&store, &operation, &request)?;
    let planned = baseline(&store, &operation)?;
    let (attempt, observed_on) = reservation(begin_at(
        &store,
        &operation,
        &request,
        &registered,
        planned,
    )?)?;
    check!(eq; (attempt, observed_on), (1, registered.observed_on.clone()));
    let result = finish(
        &store,
        &operation,
        attempt,
        Attempt::Failed(JobError::Transient {
            message: "retained source failure".into(),
        }),
        &[],
    )?;
    check!(
        matches!(result, Completion::Retry(JobError::Transient { message }) if message == "retained source failure")
    );
    store.flush()?;
    drop(store);
    let store = Store::open(root.path())?;
    let before = store.journal_keys(PHASE)?;
    for _ in 0..2 {
        check!(eq; retry(begin_at(&store, &operation, &request, &registered, planned)?)?, "retained source failure");
        check!(eq; store.journal_keys(PHASE)?, before);
    }
    let next_baseline = baseline(&store, &operation)?;
    check!(eq; next_baseline, 1);
    let (attempt, observed_on) = reservation(begin_at(
        &store,
        &operation,
        &request,
        &registered,
        next_baseline,
    )?)?;
    check!(eq; (attempt, observed_on), (2, registered.observed_on));
    check!(!store.journal_contains(PHASE, &format!("{operation}/attempt/3/reserved"))?);
    Ok(())
}

#[test]
fn postcommit_completed_and_terminal_replays_preserve_outcomes_after_reopen() -> TestResult {
    for completed in [false, true] {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let mut request = request()?;
        let operation = key(&request);
        let registered = register(&store, &operation, &request)?;
        let planned = baseline(&store, &operation)?;
        let (attempt, at) = reservation(begin_at(
            &store,
            &operation,
            &request,
            &registered,
            planned,
        )?)?;
        let effect = if completed {
            Attempt::Completed(StageOutcome {
                records: 37,
                at,
                errors: Vec::new(),
                notes: vec!["retained source progress".into()],
                disposition: census_crawl::CollectionDisposition::Complete,
                unfinished: Vec::new(),
            })
        } else {
            Attempt::Failed(JobError::Terminal {
                message: "retained terminal failure".into(),
            })
        };
        let Completion::Settled(expected) = finish(&store, &operation, attempt, effect, &[])?
        else {
            return Err("terminal physical outcome did not settle".into());
        };
        store.flush()?;
        drop(store);
        let store = Store::open(root.path())?;
        request.observed_on = "2026-10-03".into();
        let before = store.journal_keys(PHASE)?;
        let actual = settled(begin_at(
            &store,
            &operation,
            &request,
            &registered,
            planned,
        )?)?;
        check!(eq; serde_json::to_value(actual)?, serde_json::to_value(expected)?);
        check!(eq; store.journal_keys(PHASE)?, before);
    }
    Ok(())
}

#[test]
fn unknown_replay_does_not_reacquire_but_a_new_baseline_admits_a_new_slot() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let operation = key(&request);
    let registered = register(&store, &operation, &request)?;
    reservation(begin_at(&store, &operation, &request, &registered, 0)?)?;
    store.flush()?;
    drop(store);
    let store = Store::open(root.path())?;
    let before = store.journal_keys(PHASE)?;
    check!(eq; baseline(&store, &operation)?, 1);
    for _ in 0..2 {
        check!(eq; retry(begin_at(&store, &operation, &request, &registered, 0)?)?,
            "teams source attempt 1 reserved with no recorded outcome; acquisition remains uncertain");
        check!(eq; store.journal_keys(PHASE)?, before);
    }
    check!(eq; serde_json::to_value(super::inspection(&store, &operation, None)?)?,
        serde_json::json!({"status": "unsettled", "progress": [{"status": "unknown", "attempt": 1}]}));
    let planned = baseline(&store, &operation)?;
    let (attempt, observed_on) = reservation(begin_at(
        &store,
        &operation,
        &request,
        &registered,
        planned,
    )?)?;
    check!(eq; (attempt, observed_on), (2, registered.observed_on.clone()));
    check!(eq; retry(begin_at(&store, &operation, &request, &registered, planned)?)?,
        "teams source attempt 2 reserved with no recorded outcome; acquisition remains uncertain");
    check!(!store.journal_contains(PHASE, &format!("{operation}/attempt/3/reserved"))?);
    Ok(())
}

#[test]
fn an_earlier_unknown_survives_later_transient_replay_and_new_slot_admission() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let operation = key(&request);
    let registered = register(&store, &operation, &request)?;
    for expected in 1..=2 {
        let planned = baseline(&store, &operation)?;
        check!(eq; reservation(begin_at(&store, &operation, &request, &registered, planned)?)?.0, expected);
    }
    finish(
        &store,
        &operation,
        2,
        Attempt::Failed(JobError::Transient {
            message: "second source failure".into(),
        }),
        &[],
    )?;
    let before = store.journal_keys(PHASE)?;
    for _ in 0..2 {
        check!(eq; retry(begin_at(&store, &operation, &request, &registered, 1)?)?,
            "second source failure");
        check!(eq; store.journal_keys(PHASE)?, before);
    }
    let planned = baseline(&store, &operation)?;
    check!(eq; reservation(begin_at(&store, &operation, &request, &registered, planned)?)?.0, 3);
    let result = finish(
        &store,
        &operation,
        3,
        Attempt::Failed(JobError::Transient {
            message: "third source failure".into(),
        }),
        &[],
    )?;
    check!(
        matches!(result, Completion::Settled(TeamsSourceOutcome::Interrupted {
        attempts: Some(3), progress, ..
    }) if matches!(progress.first(), Some(TeamsAttemptProgress::Unknown { attempt: 1 })))
    );
    Ok(())
}

#[test]
fn final_unknown_settles_interrupted_without_an_outcome_or_fourth_slot() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let operation = key(&request);
    let registered = register(&store, &operation, &request)?;
    for expected in 1..=3 {
        let planned = baseline(&store, &operation)?;
        check!(eq; reservation(begin_at(&store, &operation, &request, &registered, planned)?)?.0, expected);
    }
    let expected = TeamsSourceOutcome::Interrupted {
        attempts: Some(3),
        message: "teams source final reservation has no recorded outcome".into(),
        progress: (1..=3)
            .map(|attempt| TeamsAttemptProgress::Unknown { attempt })
            .collect(),
    };
    for planned in [2, 3] {
        let actual = settled(begin_at(
            &store,
            &operation,
            &request,
            &registered,
            planned,
        )?)?;
        check!(eq; serde_json::to_value(actual)?, serde_json::to_value(&expected)?);
    }
    check!(!store.journal_contains(PHASE, &format!("{operation}/attempt/3/outcome"))?);
    check!(!store.journal_contains(PHASE, &format!("{operation}/attempt/4/reserved"))?);
    Ok(())
}

#[test]
fn three_known_transients_exhaust_only_after_three_distinct_baselines() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let operation = key(&request);
    let registered = register(&store, &operation, &request)?;
    for planned in 0..3 {
        check!(eq; baseline(&store, &operation)?, planned);
        let (attempt, _) = reservation(begin_at(
            &store,
            &operation,
            &request,
            &registered,
            planned,
        )?)?;
        let completion = finish(
            &store,
            &operation,
            attempt,
            Attempt::Failed(JobError::Transient {
                message: format!("failure {attempt}"),
            }),
            &[],
        )?;
        match completion {
            Completion::Retry(JobError::Transient { message }) if attempt < 3 => {
                check!(eq; message, format!("failure {attempt}"));
                check!(eq; retry(begin_at(&store, &operation, &request, &registered, planned)?)?, message);
            }
            Completion::Settled(TeamsSourceOutcome::Exhausted {
                attempts,
                last_failure,
                progress,
            }) if attempt == 3 => {
                check!(eq; attempts, 3);
                check!(eq; last_failure, "failure 3");
                check!(eq; progress.len(), 3);
                check!(progress
                    .iter()
                    .all(|item| matches!(item, TeamsAttemptProgress::Transient { .. })));
            }
            _ => return Err("known transient attempt received the wrong disposition".into()),
        }
    }
    let before = store.journal_keys(PHASE)?;
    check!(matches!(
        settled(begin_at(&store, &operation, &request, &registered, 2)?)?,
        TeamsSourceOutcome::Exhausted { attempts: 3, last_failure, .. } if last_failure == "failure 3"
    ));
    check!(eq; store.journal_keys(PHASE)?, before);
    Ok(())
}

#[test]
fn invalid_baselines_refuse_without_panicking_or_mutating_even_when_settled() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let operation = key(&request);
    let registered = register(&store, &operation, &request)?;
    for settled_history in [false, true] {
        if settled_history {
            reservation(begin_at(&store, &operation, &request, &registered, 0)?)?;
            finish(
                &store,
                &operation,
                1,
                Attempt::Failed(JobError::Terminal {
                    message: "real terminal failure".into(),
                }),
                &[],
            )?;
        }
        let before = store.journal_keys(PHASE)?;
        for planned in 4..=u8::MAX {
            check!(matches!(
                begin_at(&store, &operation, &request, &registered, planned),
                Err(JobError::Terminal { message })
                    if message == "teams source baseline is outside the persisted budget"
            ));
        }
        check!(eq; store.journal_keys(PHASE)?, before);
    }
    Ok(())
}

#[test]
fn regressed_or_skipped_baselines_refuse_without_reserving() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let operation = key(&request);
    let registered = register(&store, &operation, &request)?;
    check!(matches!(
        begin_at(&store, &operation, &request, &registered, 1),
        Err(JobError::Terminal { message })
            if message == "teams source attempt history regressed below baseline"
    ));
    for _ in 0..2 {
        let planned = baseline(&store, &operation)?;
        reservation(begin_at(
            &store,
            &operation,
            &request,
            &registered,
            planned,
        )?)?;
    }
    let before = store.journal_keys(PHASE)?;
    check!(matches!(
        begin_at(&store, &operation, &request, &registered, 0),
        Err(JobError::Terminal { message })
            if message == "teams source attempt history exceeds baseline plus one"
    ));
    check!(eq; store.journal_keys(PHASE)?, before);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn baseline_reads_use_control_admission_when_stage_work_is_saturated() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let region = Arc::new(crate::spawn::Spawner::with_capacity(1));
    let jobs = Jobs::new(Arc::clone(&store), Arc::clone(&region));
    let owner = JurisdictionCensus::new(
        Arc::clone(&store),
        Arc::new(SystemClock),
        None,
        CaptureExecution::LegacyFresh,
        jobs.clone(),
    );
    let source = TeamsSource::new(owner, jobs);
    let request = request()?;
    let operation = key(&request);
    let held = region
        .coordinator()
        .reserve(crate::admission::WORK_CLASS_STAGE)
        .await?;
    let missing = tokio::time::timeout(
        Duration::from_secs(5),
        source.ledger_baseline(operation.clone()),
    )
    .await?;
    check!(matches!(missing, Err(JobError::Terminal { .. })));
    register(&store, &operation, &request)?;
    let before = store.journal_keys(PHASE)?;
    let observed =
        tokio::time::timeout(Duration::from_secs(5), source.ledger_baseline(operation)).await??;
    check!(eq; observed, 0);
    check!(eq; store.journal_keys(PHASE)?, before);
    drop(held);
    let report = region.drain(Duration::from_secs(5)).await?;
    check!(eq; (report.accepted, report.completed, report.failed), (2, 2, 1));
    Ok(())
}
