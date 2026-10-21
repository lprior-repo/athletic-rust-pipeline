use super::*;
use crate::restate_services::jurisdiction::team_source::{key, tests::request};
use crate::restate_services::wire::StageOutcome;

type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn original(
    store: &Store,
) -> Result<(TeamsSourceRequest, String, String), Box<dyn std::error::Error>> {
    let mut request = request()?;
    request.source = "milesplit".into();
    let key = key(&request);
    let identity = super::super::register(store, &key, &request)?;
    let admission = super::super::begin_at(store, &key, &request, &identity, 0)?;
    check!(matches!(
        admission,
        AdmissionReplay::Reserved { attempt: 1, .. }
    ));
    super::super::finish(store, &key, 1, Attempt::Failed(JobError::Terminal {
        message: "origin https://oh.milesplit.com is held by another census-service process (holder: {\"pid\":424242,\"origin\":\"https://oh.milesplit.com\"})".into(),
    }), &[])?;
    let outcome = super::super::status(store, &key, Some(&request))?.ok_or("original absent")?;
    Ok((request, key, serialized_digest(&outcome)?))
}

#[test]
fn explicit_continuation_completes_on_global_attempt_two_and_keeps_original_receipt() -> TestResult
{
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (request, key, digest) = original(&store)?;
    let original_keys = store.journal_keys(super::super::PHASE)?;
    let authorization = authorize(&store, &key, &request, &digest)?;
    check!(eq; baseline(&store, &key)?, 1);
    check!(matches!(
        begin_at(&store, &key, &request, &authorization, 1)?,
        AdmissionReplay::Reserved { attempt: 2, .. }
    ));
    let outcome = StageOutcome {
        records: 7,
        at: request.observed_on.clone(),
        errors: Vec::new(),
        notes: Vec::new(),
        disposition: census_crawl::CollectionDisposition::Complete,
        unfinished: Vec::new(),
    };
    check!(matches!(
        finish(&store, &key, 2, Attempt::Completed(outcome), &[])?,
        Completion::Settled(TeamsSourceOutcome::Completed { .. })
    ));
    store.flush()?;
    drop(store);
    let store = Store::open(root.path())?;
    check!(matches!(
        begin_at(&store, &key, &request, &authorization, 1)?,
        AdmissionReplay::Settled(TeamsSourceOutcome::Completed { .. })
    ));
    let replayed = super::super::status(&store, &key, Some(&request))?.ok_or("original lost")?;
    check!(eq; serialized_digest(&replayed)?, digest);
    check!(eq; store.journal_keys(super::super::PHASE)?, original_keys);
    let counts = super::super::durable_attempts(&store)?;
    check!(eq; (counts.sources, counts.reserved, counts.recorded, counts.settled, counts.unknown), (1, 2, 2, 1, 0));
    check!(!store.journal_contains(PHASE, &slot_key(&key, 3, "reserved"))?);
    Ok(())
}

#[test]
fn unknown_continuation_attempt_is_consumed_and_never_represented_as_three_failures() -> TestResult
{
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (request, key, digest) = original(&store)?;
    let authorization = authorize(&store, &key, &request, &digest)?;
    check!(matches!(
        begin_at(&store, &key, &request, &authorization, 1)?,
        AdmissionReplay::Reserved { attempt: 2, .. }
    ));
    store.flush()?;
    drop(store);
    let store = Store::open(root.path())?;
    check!(matches!(
        begin_at(&store, &key, &request, &authorization, 1)?,
        AdmissionReplay::ReplayRetry { .. }
    ));
    check!(eq; baseline(&store, &key)?, 2);
    check!(matches!(
        begin_at(&store, &key, &request, &authorization, 2)?,
        AdmissionReplay::Reserved { attempt: 3, .. }
    ));
    check!(matches!(
        finish(
            &store,
            &key,
            3,
            Attempt::Failed(JobError::Transient {
                message: "network unavailable".into()
            }),
            &[]
        )?,
        Completion::Settled(TeamsSourceOutcome::Interrupted {
            attempts: Some(3),
            ..
        })
    ));
    check!(matches!(
        begin_at(&store, &key, &request, &authorization, 3)?,
        AdmissionReplay::Settled(TeamsSourceOutcome::Interrupted { .. })
    ));
    check!(!store.journal_contains(PHASE, &slot_key(&key, 4, "reserved"))?);
    Ok(())
}
