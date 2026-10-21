use super::*;
use crate::restate_services::jurisdiction::team_source::{key, tests::request};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn terminal_original(
    store: &Store,
    message: String,
) -> Result<(TeamsSourceRequest, String, String), Box<dyn std::error::Error>> {
    let mut request = request()?;
    request.source = "milesplit".into();
    let key = key(&request);
    let identity = super::super::register(store, &key, &request)?;
    super::super::begin_at(store, &key, &request, &identity, 0)?;
    super::super::finish(
        store,
        &key,
        1,
        Attempt::Failed(JobError::Terminal { message }),
        &[],
    )?;
    let outcome = super::super::status(store, &key, None)?.ok_or("original absent")?;
    Ok((request, key, serialized_digest(&outcome)?))
}

#[test]
fn ownership_lookalikes_and_access_refusals_never_create_continuation_authority() -> TestResult {
    let messages = [
        "HTTP 403 public source refused",
        "challenge required for https://oh.milesplit.com",
        "origin https://oh.milesplit.com is held by another census-service process (holder: not-json)",
        "origin https://oh.milesplit.com is held by another census-service process (holder: {\"pid\":0,\"origin\":\"https://oh.milesplit.com\"})",
        "origin https://oh.milesplit.com is held by another census-service process (holder: {\"pid\":1,\"origin\":\"https://mi.milesplit.com\"})",
        "origin https://mi.milesplit.com is held by another census-service process (holder: {\"pid\":1,\"origin\":\"https://mi.milesplit.com\"})",
    ];
    for message in messages {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let (request, key, digest) = terminal_original(&store, message.into())?;
        let original_keys = store.journal_keys(super::super::PHASE)?;
        check!(
            authorize(&store, &key, &request, &digest).is_err(),
            "accepted {message}"
        );
        check!(store.journal_keys(PHASE)?.is_empty());
        check!(eq; store.journal_keys(super::super::PHASE)?, original_keys);
        check!(eq; serialized_digest(&super::super::status(&store, &key, None)?)?, serialized_digest(&super::super::status(&store, &key, Some(&request))?)?);
    }
    Ok(())
}

#[test]
fn wrong_digest_or_request_is_read_only_and_exact_authorization_is_idempotent() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (request, key, digest) = super::tests::original(&store)?;
    check!(authorize(&store, &key, &request, "wrong-digest").is_err());
    let mut changed = request.clone();
    changed.jurisdiction.revision = census_domain::model::Revision(2);
    check!(authorize(&store, &key, &changed, &digest).is_err());
    changed = request.clone();
    changed.source = "ihsa".into();
    check!(authorize(&store, &key, &changed, &digest).is_err());
    check!(store.journal_keys(PHASE)?.is_empty());
    let authorization = authorize(&store, &key, &request, &digest)?;
    let keys = store.journal_keys(PHASE)?;
    check!(eq; authorize(&store, &key, &request, &digest)?, authorization);
    check!(eq; store.journal_keys(PHASE)?, keys);
    check!(eq; baseline(&store, &key)?, 1);
    Ok(())
}

#[test]
fn acknowledged_failures_stop_at_the_original_global_ceiling() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (request, key, digest) = super::tests::original(&store)?;
    let authorization = authorize(&store, &key, &request, &digest)?;
    for attempt in 2..=3 {
        check!(
            matches!(begin_at(&store, &key, &request, &authorization, attempt - 1)?, AdmissionReplay::Reserved { attempt: reserved, .. } if reserved == attempt)
        );
        let result = finish(
            &store,
            &key,
            attempt,
            Attempt::Failed(JobError::Transient {
                message: format!("timeout {attempt}"),
            }),
            &[],
        )?;
        if attempt == 2 {
            check!(matches!(
                result,
                Completion::Retry(JobError::Transient { .. })
            ));
        } else {
            check!(matches!(
                result,
                Completion::Settled(TeamsSourceOutcome::Exhausted { attempts: 3, .. })
            ));
        }
    }
    check!(matches!(
        begin_at(&store, &key, &request, &authorization, 3)?,
        AdmissionReplay::Settled(TeamsSourceOutcome::Exhausted { attempts: 3, .. })
    ));
    check!(!store.journal_contains(PHASE, &slot_key(&key, 4, "reserved"))?);
    check!(eq; serialized_digest(&super::super::status(&store, &key, None)?.ok_or("original lost")?)?, digest);
    Ok(())
}

#[test]
fn orphan_reservation_is_refused_before_authorization() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (request, key, digest) = super::tests::original(&store)?;
    let mut batch = store.write_batch();
    batch.journal_done(
        PHASE,
        &slot_key(&key, 2, "reserved"),
        &Reservation {
            attempt: 2,
            observed_on: request.observed_on.clone(),
        },
    )?;
    batch.commit()?;
    check!(status(&store, &key).is_err());
    check!(authorize(&store, &key, &request, &digest).is_err());
    check!(!store.journal_contains(PHASE, &format!("{key}/authorization"))?);
    Ok(())
}

#[test]
fn observed_on_mismatch_is_rejected_before_and_after_authorization() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (request, key, digest) = super::tests::original(&store)?;
    let original_keys = store.journal_keys(super::super::PHASE)?;
    let original_payloads = store.journal_payloads(super::super::PHASE)?;

    let before = store.snapshot().sequence();
    let mut wrong_date = request.clone();
    wrong_date.observed_on = "2025-01-01T00:00:00Z".into();
    check!(authorize(&store, &key, &wrong_date, &digest).is_err());
    check!(store.journal_keys(PHASE)?.is_empty());
    check!(eq; store.snapshot().sequence(), before);
    check!(eq; store.journal_keys(super::super::PHASE)?, original_keys);
    check!(eq; store.journal_payloads(super::super::PHASE)?, original_payloads);

    let authorization = authorize(&store, &key, &request, &digest)?;
    let auth_keys = store.journal_keys(PHASE)?;
    let auth_payloads = store.journal_payloads(PHASE)?;
    check!(authorize(&store, &key, &wrong_date, &digest).is_err());
    check!(eq; store.journal_keys(PHASE)?, auth_keys);
    check!(eq; store.journal_payloads(PHASE)?, auth_payloads);
    check!(eq; store.journal_keys(super::super::PHASE)?, original_keys);
    check!(eq; store.journal_payloads(super::super::PHASE)?, original_payloads);

    store.flush()?;
    drop(store);
    let store = Store::open(root.path())?;
    check!(authorize(&store, &key, &wrong_date, &digest).is_err());
    check!(eq; authorize(&store, &key, &request, &digest)?, authorization);
    check!(eq; store.journal_keys(super::super::PHASE)?, original_keys);
    check!(eq; store.journal_payloads(super::super::PHASE)?, original_payloads);
    Ok(())
}
