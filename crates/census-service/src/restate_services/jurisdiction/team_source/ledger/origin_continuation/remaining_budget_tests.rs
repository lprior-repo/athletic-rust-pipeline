use super::*;
use crate::restate_services::jurisdiction::team_source::{key, tests::request};
use crate::restate_services::wire::StageOutcome;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const OWNERSHIP_REFUSAL: &str = "origin https://oh.milesplit.com is held by another census-service process (holder: {\"pid\":424242,\"origin\":\"https://oh.milesplit.com\"})";

enum OriginalHistory {
    OwnershipRefusal { attempt: u8 },
    Exhausted,
    Interrupted,
}

fn seed(
    store: &Store,
    history: OriginalHistory,
) -> TestResult<(TeamsSourceRequest, String, String)> {
    let mut request = request()?;
    request.source = "milesplit".into();
    let key = key(&request);
    let identity = super::super::register(store, &key, &request)?;
    let last = match history {
        OriginalHistory::OwnershipRefusal { attempt } => attempt,
        OriginalHistory::Exhausted | OriginalHistory::Interrupted => MAX_ATTEMPTS,
    };
    for attempt in 1..=last {
        check!(matches!(
            super::super::begin_at(store, &key, &request, &identity, attempt - 1)?,
            AdmissionReplay::Reserved { attempt: reserved, .. } if reserved == attempt
        ));
        if matches!(history, OriginalHistory::Interrupted) && attempt == 1 {
            continue;
        }
        let error = match history {
            OriginalHistory::OwnershipRefusal { attempt: terminal } if terminal == attempt => {
                JobError::Terminal {
                    message: OWNERSHIP_REFUSAL.into(),
                }
            }
            _ => JobError::Transient {
                message: format!("recorded original timeout {attempt}"),
            },
        };
        super::super::finish(store, &key, attempt, Attempt::Failed(error), &[])?;
    }
    let original = super::super::status(store, &key, Some(&request))?
        .ok_or("seed did not retain an original settlement")?;
    Ok((request, key, serialized_digest(&original)?))
}

fn original_rows(store: &Store) -> TestResult<Vec<(String, serde_json::Value)>> {
    let mut keys: Vec<_> = store
        .journal_keys(super::super::PHASE)?
        .into_iter()
        .collect();
    keys.sort();
    let snapshot = store.snapshot();
    keys.into_iter()
        .map(|key| {
            let value = snapshot
                .journal_payload(super::super::PHASE, &key)?
                .ok_or("original journal row disappeared")?;
            Ok((key, value))
        })
        .collect()
}

#[test]
fn two_original_attempts_leave_only_global_attempt_three_across_cold_replay() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (request, key, digest) = seed(&store, OriginalHistory::OwnershipRefusal { attempt: 2 })?;
    let original = original_rows(&store)?;
    let authorization = authorize(&store, &key, &request, &digest)?;
    check!(eq; baseline(&store, &key)?, 2);
    check!(matches!(
        begin_at(&store, &key, &request, &authorization, 2)?,
        AdmissionReplay::Reserved { attempt: 3, .. }
    ));
    let result = finish(
        &store,
        &key,
        3,
        Attempt::Failed(JobError::Transient {
            message: "only remaining continuation attempt failed".into(),
        }),
        &[],
    )?;
    check!(matches!(
        result,
        Completion::Settled(TeamsSourceOutcome::Exhausted { attempts: 3, .. })
    ));
    store.flush()?;
    drop(store);
    let store = Store::open(root.path())?;
    for baseline in [2, 3] {
        check!(matches!(
            begin_at(&store, &key, &request, &authorization, baseline)?,
            AdmissionReplay::Settled(TeamsSourceOutcome::Exhausted { attempts: 3, .. })
        ));
    }
    check!(eq; original_rows(&store)?, original);
    let replay = super::super::status(&store, &key, Some(&request))?
        .ok_or("original refusal disappeared")?;
    check!(eq; serialized_digest(&replay)?, digest);
    let counts = super::super::durable_attempts(&store)?;
    check!(eq; (counts.sources, counts.reserved, counts.recorded, counts.settled, counts.unknown), (1, 3, 3, 1, 0));
    check!(!store.journal_contains(PHASE, &slot_key(&key, 2, "reserved"))?);
    check!(!store.journal_contains(PHASE, &slot_key(&key, 4, "reserved"))?);
    Ok(())
}

#[test]
fn successful_last_global_slot_retains_both_original_attempts() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (request, key, digest) = seed(&store, OriginalHistory::OwnershipRefusal { attempt: 2 })?;
    let original = original_rows(&store)?;
    let authorization = authorize(&store, &key, &request, &digest)?;
    begin_at(&store, &key, &request, &authorization, 2)?;
    let result = finish(
        &store,
        &key,
        3,
        Attempt::Completed(StageOutcome {
            records: 11,
            at: request.observed_on.clone(),
            errors: Vec::new(),
            notes: Vec::new(),
            disposition: census_crawl::CollectionDisposition::Complete,
            unfinished: Vec::new(),
        }),
        &[],
    )?;
    let Completion::Settled(TeamsSourceOutcome::Completed { outcome, progress }) = result else {
        return Err("last global attempt did not complete".into());
    };
    check!(eq; outcome.records, 11);
    check!(eq; progress.len(), 3);
    check!(matches!(
        progress.first(),
        Some(TeamsAttemptProgress::Transient { attempt: 1, .. })
    ));
    check!(
        matches!(progress.get(1), Some(TeamsAttemptProgress::Terminal { attempt: 2, message, .. }) if message == OWNERSHIP_REFUSAL)
    );
    check!(matches!(
        progress.last(),
        Some(TeamsAttemptProgress::Completed { attempt: 3, .. })
    ));
    check!(eq; original_rows(&store)?, original);
    check!(matches!(
        begin_at(&store, &key, &request, &authorization, 3)?,
        AdmissionReplay::Settled(TeamsSourceOutcome::Completed { .. })
    ));
    check!(!store.journal_contains(PHASE, &slot_key(&key, 4, "reserved"))?);
    Ok(())
}

#[test]
fn ownership_refusal_on_original_attempt_three_cannot_authorize_more_work() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (request, key, digest) = seed(&store, OriginalHistory::OwnershipRefusal { attempt: 3 })?;
    let original = original_rows(&store)?;
    check!(authorize(&store, &key, &request, &digest).is_err());
    check!(store.journal_keys(PHASE)?.is_empty());
    check!(eq; original_rows(&store)?, original);
    let counts = super::super::durable_attempts(&store)?;
    check!(eq; (counts.sources, counts.reserved, counts.recorded, counts.settled), (1, 3, 3, 1));
    Ok(())
}

#[test]
fn original_exhaustion_and_interruption_are_not_ownership_continuation_authority() -> TestResult {
    for history in [OriginalHistory::Exhausted, OriginalHistory::Interrupted] {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let interrupted = matches!(history, OriginalHistory::Interrupted);
        let (request, key, digest) = seed(&store, history)?;
        let original = original_rows(&store)?;
        let outcome =
            super::super::status(&store, &key, Some(&request))?.ok_or("original outcome absent")?;
        if interrupted {
            check!(matches!(
                outcome,
                TeamsSourceOutcome::Interrupted {
                    attempts: Some(3),
                    ..
                }
            ));
        } else {
            check!(matches!(
                outcome,
                TeamsSourceOutcome::Exhausted { attempts: 3, .. }
            ));
        }
        check!(authorize(&store, &key, &request, &digest).is_err());
        check!(store.journal_keys(PHASE)?.is_empty());
        check!(eq; original_rows(&store)?, original);
    }
    Ok(())
}
