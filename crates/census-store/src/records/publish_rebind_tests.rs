use super::*;
use crate::publish::Root;
use crate::records::codec::encode;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn digest(value: char) -> StoreResult<Digest> {
    Digest::parse(&value.to_string().repeat(64))
}

fn fixture() -> TestResult<(tempfile::TempDir, Store, Intent)> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let identity = digest('a')?;
    let epoch = store.lease_acquire(identity.as_str(), "workbook-publication", "2999-12-31")?;
    let request = store.publication_request(Request {
        namespace: digest('b')?,
        identity,
        expected: Root::Bundle(digest('c')?),
        input: InputFence {
            lineage: digest('d')?,
            token: digest('e')?,
            recipe: digest('f')?,
            run_manifest: None,
            evidence_generation: store.evidence_generation(),
            derived_generation: store.derived_generation(),
        },
        lease_epoch: epoch,
    })?;
    let intent = store.publication_mark(
        &request,
        State::Intent {
            desired: digest('1')?,
        },
    )?;
    Ok((directory, store, intent))
}

fn expire(store: &Store, intent: &Intent) -> TestResult {
    let mut lease = store
        .get_lease_record(intent.request.identity.as_str())?
        .ok_or("lease missing")?;
    lease.expires_at = "2000-01-01".into();
    store.meta.insert(
        format!("lease/{}", intent.request.identity.as_str()),
        encode(&lease)?,
    )?;
    store.db.persist(fjall::PersistMode::SyncData)?;
    Ok(())
}

fn retained_tuple(
    store: &Store,
    intent: &Intent,
) -> TestResult<(Intent, Intent, crate::records::model::LeaseRecord, u64)> {
    Ok((
        store
            .publication_read(&intent.request.identity)?
            .ok_or("request missing")?,
        store
            .publication_active(&intent.request.namespace)?
            .ok_or("active missing")?,
        store
            .get_lease_record(intent.request.identity.as_str())?
            .ok_or("lease missing")?,
        crate::meta::get_u64(&store.meta, "lease_epoch_counter")?.ok_or("counter missing")?,
    ))
}

#[test]
fn abandoned_publication_rebind_batch_preserves_the_entire_old_tuple() -> TestResult {
    let (directory, store, intent) = fixture()?;
    expire(&store, &intent)?;
    let before = retained_tuple(&store, &intent)?;
    {
        let _writes = store.lock_appends();
        let mut batch = store.db.batch();
        let epoch = store.lease_succeed_expired_stage(
            intent.request.identity.as_str(),
            "workbook-publication",
            "2999-12-31",
            intent.request.lease_epoch,
            &mut batch,
        )?;
        let mut updated = intent.clone();
        updated.request.lease_epoch = epoch;
        stage(
            &store,
            &updated,
            Some(&updated.request.namespace),
            &mut batch,
        )?;
        check!(eq; retained_tuple(&store, &intent)?, before);
    }
    drop(store);
    let reopened = Store::open(directory.path())?;
    check!(eq; retained_tuple(&reopened, &intent)?, before);
    Ok(())
}

#[test]
fn committed_publication_rebind_reopens_one_matching_successor_tuple() -> TestResult {
    let (directory, store, intent) = fixture()?;
    expire(&store, &intent)?;
    let updated = store.publication_rebind_lease(&intent.request)?;
    check!(updated.request.lease_epoch > intent.request.lease_epoch);
    check!(eq; updated.state, intent.state);
    drop(store);
    let reopened = Store::open(directory.path())?;
    let (request, active, lease, counter) = retained_tuple(&reopened, &intent)?;
    check!(eq; request, updated);
    check!(eq; active, updated);
    check!(eq; lease.epoch, updated.request.lease_epoch);
    check!(eq; counter, updated.request.lease_epoch);
    reopened.lease_check(
        intent.request.identity.as_str(),
        updated.request.lease_epoch,
    )?;
    Ok(())
}

#[test]
fn stale_publication_owners_cannot_mutate_a_rebound_intent() -> TestResult {
    let (_directory, store, intent) = fixture()?;
    expire(&store, &intent)?;
    let updated = store.publication_rebind_lease(&intent.request)?;
    let before = retained_tuple(&store, &intent)?;
    check!(matches!(
        store.publication_rebind_lease(&intent.request),
        Err(StoreError::RecordConflict { .. })
    ));
    check!(matches!(
        store.publication_fence(&intent.request, &None),
        Err(StoreError::RecordConflict { .. })
    ));
    check!(matches!(
        store.lease_release(intent.request.identity.as_str(), intent.request.lease_epoch),
        Err(StoreError::LeaseMismatch { .. })
    ));
    [
        State::Published {
            receipt: digest('1')?,
        },
        State::Superseded {
            desired: digest('1')?,
        },
    ]
    .into_iter()
    .try_for_each(|state| -> TestResult {
        check!(matches!(
            store.publication_mark(&intent, state),
            Err(StoreError::RecordConflict { .. })
        ));
        Ok(())
    })?;
    {
        let fence = store.publication_fence(&updated.request, &None)?;
        check!(matches!(
            fence.begin(&intent, &digest('1')?),
            Err(StoreError::RecordConflict { .. })
        ));
        check!(matches!(
            fence.published(&intent, &digest('1')?),
            Err(StoreError::RecordConflict { .. })
        ));
    }
    check!(eq; retained_tuple(&store, &intent)?, before);
    Ok(())
}

#[test]
fn expired_publication_fence_exposes_the_authoritative_intent() -> TestResult {
    let (_directory, store, intent) = fixture()?;
    expire(&store, &intent)?;
    let published = {
        let fence = store.publication_fence(&intent.request, &None)?;
        check!(fence.intent().request.lease_epoch > intent.request.lease_epoch);
        let begun = fence.begin(fence.intent(), &digest('1')?)?;
        fence.published(&begun, &digest('1')?)?
    };
    check!(eq; published.state, State::Published { receipt: digest('1')? });
    check!(eq; store.publication_active(&intent.request.namespace)?, Some(published));
    Ok(())
}

#[test]
fn wrong_expected_root_refuses_before_expired_lease_succession() -> TestResult {
    let (_directory, store, intent) = fixture()?;
    expire(&store, &intent)?;
    let before = retained_tuple(&store, &intent)?;
    let mut altered = intent.request.clone();
    altered.expected = Root::Absent;
    check!(matches!(
        store.publication_rebind_lease(&altered),
        Err(StoreError::RecordConflict { .. })
    ));
    check!(matches!(
        store.publication_fence(&altered, &None),
        Err(StoreError::RecordConflict { .. })
    ));
    check!(eq; retained_tuple(&store, &intent)?, before);
    Ok(())
}

#[test]
fn live_lease_and_exhausted_epoch_refuse_without_partial_publication_writes() -> TestResult {
    let (_directory, store, intent) = fixture()?;
    let before = retained_tuple(&store, &intent)?;
    check!(matches!(
        store.publication_rebind_lease(&intent.request),
        Err(StoreError::LeaseMismatch { .. })
    ));
    check!(eq; retained_tuple(&store, &intent)?, before);
    expire(&store, &intent)?;
    store
        .meta
        .insert("lease_epoch_counter", u64::MAX.to_string().as_bytes())?;
    let exhausted = retained_tuple(&store, &intent)?;
    check!(matches!(
        store.publication_rebind_lease(&intent.request),
        Err(StoreError::CounterExhausted)
    ));
    check!(eq; retained_tuple(&store, &intent)?, exhausted);
    Ok(())
}

#[test]
fn visible_bundle_finalization_allows_exact_expired_epoch_but_refuses_torn_ownership() -> TestResult
{
    let (_directory, store, intent) = fixture()?;
    expire(&store, &intent)?;
    let published = State::Published {
        receipt: digest('1')?,
    };
    let mut successor = store
        .get_lease_record(intent.request.identity.as_str())?
        .ok_or("lease missing")?;
    successor.epoch = successor.epoch.checked_add(1).ok_or("epoch exhausted")?;
    store.meta.insert(
        format!("lease/{}", intent.request.identity.as_str()),
        encode(&successor)?,
    )?;
    let before = retained_tuple(&store, &intent)?;
    check!(matches!(
        store.publication_mark(&intent, published.clone()),
        Err(StoreError::LeaseMismatch { .. })
    ));
    check!(eq; retained_tuple(&store, &intent)?, before);
    check!(store
        .meta
        .get(format!(
            "publication/v1/receipt/{}",
            intent.request.identity.as_str()
        ))?
        .is_none());
    successor.epoch = intent.request.lease_epoch;
    store.meta.insert(
        format!("lease/{}", intent.request.identity.as_str()),
        encode(&successor)?,
    )?;
    let completed = store.publication_mark(&intent, published.clone())?;
    check!(eq; completed.state, published);
    store.lease_release(intent.request.identity.as_str(), intent.request.lease_epoch)?;
    check!(eq; store.publication_mark(&completed, published)?, completed);
    Ok(())
}

#[test]
fn missing_publication_lease_cannot_create_a_published_receipt() -> TestResult {
    let (_directory, store, intent) = fixture()?;
    store.lease_release(intent.request.identity.as_str(), intent.request.lease_epoch)?;
    check!(matches!(
        store.publication_mark(
            &intent,
            State::Published {
                receipt: digest('1')?
            }
        ),
        Err(StoreError::LeaseMismatch { actual: None, .. })
    ));
    check!(eq; store.publication_read(&intent.request.identity)?, Some(intent.clone()));
    check!(eq; store.publication_active(&intent.request.namespace)?, Some(intent.clone()));
    check!(store
        .meta
        .get(format!(
            "publication/v1/receipt/{}",
            intent.request.identity.as_str()
        ))?
        .is_none());
    Ok(())
}

#[test]
fn published_receipt_survives_atomic_rebind_and_reopen() -> TestResult {
    let (directory, store, intent) = fixture()?;
    let published = store.publication_mark(
        &intent,
        State::Published {
            receipt: digest('1')?,
        },
    )?;
    expire(&store, &published)?;
    let updated = store.publication_rebind_lease(&published.request)?;
    check!(updated.request.lease_epoch > published.request.lease_epoch);
    drop(store);
    let reopened = Store::open(directory.path())?;
    let (request, active, lease, counter) = retained_tuple(&reopened, &intent)?;
    check!(eq; request, updated);
    check!(eq; active, updated);
    check!(eq; lease.epoch, updated.request.lease_epoch);
    check!(eq; counter, updated.request.lease_epoch);
    let receipt = reopened
        .meta
        .get(format!(
            "publication/v1/receipt/{}",
            intent.request.identity.as_str()
        ))?
        .ok_or("receipt missing")?;
    check!(eq; receipt.as_ref(), digest('1')?.as_str().as_bytes());
    Ok(())
}

#[test]
fn publication_snapshot_is_captured_under_the_pointer_switch_write_fence() -> TestResult {
    let (_directory, store, intent) = fixture()?;
    let fence = store.publication_fence(&intent.request, &None)?;
    let snapshot = fence.snapshot();
    check!(store.appends.try_lock().is_err());
    check!(eq; snapshot.evidence_generation(), intent.request.input.evidence_generation);
    check!(eq; snapshot.derived_generation(), intent.request.input.derived_generation);
    drop(snapshot);
    drop(fence);
    check!(store.appends.try_lock().is_ok());
    store.journal_done("publication-fence-snapshot-test", "after-release", &true)?;
    Ok(())
}
