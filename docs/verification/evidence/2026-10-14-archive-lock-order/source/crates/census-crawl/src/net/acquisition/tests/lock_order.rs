use super::*;
use crate::net::acquisition::CaptureInterruption;
use crate::net::cache::archive_lock_test::{self, LockEvent};
use crate::net::cache::{content_digest, CacheMeta, Capture, EligibleCacheV2};
use crate::net::FetchError;
use std::cell::RefCell;
use std::io::ErrorKind;
use std::path::Path;
use std::rc::Rc;

async fn interrupted_fixture(
    previous: Option<&[u8]>,
    status: u16,
    interruption: CaptureInterruption,
) -> TestResult<Fixture> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    let requests = Arc::clone(&fixture.requests);
    let reply = Reply {
        status,
        body: BODY,
        headers: "",
        before: Mutation::None,
    };
    if let Some(previous) = previous {
        let fetch = async {
            fixture.acquire(2025, true).await??;
            fixture.fetcher.capture_interrupt = interruption;
            let result = fixture.acquire(2026, true).await?;
            check!(matches!(result, Err(ref error) if injected(error)));
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        };
        tokio::try_join!(
            serve(listener, [Reply::ok(previous), reply], requests),
            fetch
        )?;
    } else {
        fixture.fetcher.capture_interrupt = interruption;
        let fetch = async {
            let result = fixture.acquire(2026, true).await?;
            check!(matches!(result, Err(ref error) if injected(error)));
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        };
        tokio::try_join!(serve(listener, [reply], requests), fetch)?;
    }
    fixture.resume_capture();
    fixture.reopen()
}

fn observe_recovery(
    meta_path: &Path,
    events: Rc<RefCell<Vec<LockEvent>>>,
) -> Result<archive_lock_test::Observation, FetchError> {
    let eligible = meta_path.with_extension("json.lock");
    archive_lock_test::observe(move |path, event| {
        if event == LockEvent::BeforeAcquire {
            if !events.borrow().is_empty() {
                return Err(FetchError::Cache {
                    path: path.to_path_buf(),
                    source: std::io::Error::other("recursive archive acquisition during recovery"),
                });
            }
            archive_lock_test::require_available(&eligible)?;
        } else {
            archive_lock_test::require_held(path)?;
        }
        if event == LockEvent::EligibleAcquired {
            archive_lock_test::require_held(&eligible)?;
        }
        events.borrow_mut().push(event);
        Ok(())
    })
}

fn archived_metadata(root: &Path, metadata: &CacheMeta) -> TestResult<std::path::PathBuf> {
    let capture = Capture::from_meta(metadata, root)?;
    Ok(root
        .join("archive/captures")
        .join(&metadata.content_digest)
        .join(format!("{}.meta.json", content_digest(&capture.encoded))))
}

fn receipts(fixture: &Fixture) -> TestResult<Vec<census_store::Receipt>> {
    let mut receipts = Vec::new();
    fixture.store.for_each_receipt(64, |receipt| {
        receipts.push(receipt);
        Ok(())
    })?;
    Ok(receipts)
}

#[tokio::test]
async fn archive_lock_order_current_v1_recovery_publishes_under_one_archive_guard() -> TestResult {
    let mut previous = BODY.to_vec();
    previous.push(b'\n');
    for prior in [None, Some(previous.as_slice())] {
        let fixture = interrupted_fixture(prior, 200, CaptureInterruption::AfterArchive).await?;
        let (body_path, meta_path) = fixture.cache_paths()?;
        let old = if prior.is_some() {
            let envelope: EligibleCacheV2 = serde_json::from_slice(&std::fs::read(&meta_path)?)?;
            let path = archived_metadata(&fixture.store.http_cache_dir(), &envelope.metadata)?;
            Some((path.clone(), std::fs::read(path)?))
        } else {
            None
        };
        let requests = fixture.count();
        let events = Rc::new(RefCell::new(Vec::new()));
        let observation = observe_recovery(&meta_path, Rc::clone(&events))?;
        let recovered = fixture.acquire(2026, true).await??;
        drop(observation);
        check!(eq; *events.borrow(), vec![LockEvent::BeforeAcquire, LockEvent::Acquired, LockEvent::EligibleAcquired]);
        check!(eq; Fixture::occurrence(&recovered)?.attempt().ordinal().get(), 1);
        check!(eq; recovered.capture.fetched_at, CAPTURED);
        check!(eq; recovered.capture.body, BODY);
        check!(matches!(
            recovered.verdict,
            crate::milesplit::OwnedMeetVerdict::Parsed(_)
        ));
        check!(eq; fixture.count(), requests);
        let envelope: EligibleCacheV2 = serde_json::from_slice(&std::fs::read(&meta_path)?)?;
        check!(eq; &envelope.original_occurrence, Fixture::occurrence(&recovered)?);
        check!(eq; std::fs::read(&meta_path)?, envelope.encode(&meta_path)?);
        check!(eq; std::fs::read(&body_path)?, BODY);
        let retained = fixture.store.read_retained_acquisition_v1(
            Fixture::occurrence(&recovered)?,
            &mut census_store::AcquisitionReadBudget::for_work(),
        )?;
        check!(eq; retained.prepared().metadata_bytes(), std::fs::read(archived_metadata(&fixture.store.http_cache_dir(), &envelope.metadata)?)?);
        if let Some((path, bytes)) = old {
            check!(eq; std::fs::read(path)?, bytes);
            check!(eq; std::fs::read(fixture.store.http_cache_dir().join("archive/bodies").join(format!("{}.body", content_digest(&previous))))?, previous);
        }
        let journal = fixture
            .store
            .journal_payloads(census_domain::model::SOURCE_ACQUISITION_PHASE)?;
        let receipts_before = receipts(&fixture)?;
        let fixture = fixture.reopen()?;
        let replay = fixture.acquire(2026, true).await??;
        check!(eq; Fixture::occurrence(&replay)?, Fixture::occurrence(&recovered)?);
        check!(eq; fixture.store.journal_payloads(census_domain::model::SOURCE_ACQUISITION_PHASE)?, journal);
        check!(eq; receipts(&fixture)?, receipts_before);
        check!(eq; fixture.count(), requests);
        archive_lock_test::require_available(
            &fixture.store.http_cache_dir().join("archive-capacity.lock"),
        )?;
        archive_lock_test::require_available(&meta_path.with_extension("json.lock"))?;
    }
    Ok(())
}

#[tokio::test]
async fn archive_lock_order_prepared_missing_or_corrupt_backing_defers_admission() -> TestResult {
    for interruption in [
        CaptureInterruption::AfterPrepared,
        CaptureInterruption::AfterArchive,
    ] {
        let fixture = interrupted_fixture(None, 200, interruption).await?;
        let root = fixture.store.http_cache_dir();
        let record = root.join("archive/reservations").join(format!(
            "{}-{}",
            "a".repeat(64),
            "b".repeat(64)
        ));
        std::fs::create_dir_all(record.parent().ok_or("reservation parent")?)?;
        std::fs::write(&record, b"malformed")?;
        if interruption == CaptureInterruption::AfterArchive {
            let body = root
                .join("archive/bodies")
                .join(format!("{}.body", content_digest(BODY)));
            std::fs::remove_file(&body)?;
            std::fs::write(body, b"corrupt body")?;
        }
        let before = fixture
            .store
            .journal_payloads(census_domain::model::SOURCE_ACQUISITION_PHASE)?;
        let receipts_before = receipts(&fixture)?;
        let result = fixture.acquire(2026, true).await?;
        let Err(crate::CrawlError::Fetch(FetchError::CaptureStore(error))) = result else {
            return Err("expected existing Store backing refusal before archive admission".into());
        };
        match interruption {
            CaptureInterruption::AfterPrepared => check!(matches!(
                error.as_ref(),
                census_store::StoreError::Capture(
                    census_store::CaptureError::RequiredBackingMissing
                )
            )),
            CaptureInterruption::AfterArchive => check!(matches!(
                error.as_ref(),
                census_store::StoreError::Capture(census_store::CaptureError::BackingIntegrity)
            )),
            _ => return Err("unexpected fixture interruption".into()),
        }
        check!(eq; fixture.store.journal_payloads(census_domain::model::SOURCE_ACQUISITION_PHASE)?, before);
        check!(eq; receipts(&fixture)?, receipts_before);
        check!(eq; std::fs::read(&record)?, b"malformed");
        check!(eq; fixture.count(), 1);
        if interruption == CaptureInterruption::AfterPrepared {
            check!(!root.join("archive/bodies").exists());
            check!(!root.join("archive/captures").exists());
        }
        archive_lock_test::require_available(&root.join("archive-capacity.lock"))?;
    }
    Ok(())
}

#[tokio::test]
async fn archive_lock_order_non200_prepared_recovery_never_runs_archive_admission() -> TestResult {
    let fixture = interrupted_fixture(None, 500, CaptureInterruption::AfterArchive).await?;
    let record = fixture
        .store
        .http_cache_dir()
        .join("archive/reservations")
        .join(format!("{}-{}", "a".repeat(64), "b".repeat(64)));
    std::fs::write(&record, b"malformed")?;
    let result = fixture.acquire(2026, true).await?;
    check!(matches!(
        result,
        Err(crate::CrawlError::Fetch(FetchError::Http {
            status: 500,
            ..
        }))
    ));
    check!(eq; std::fs::read(&record)?, b"malformed");
    check!(eq; fixture.count(), 1);
    let (body_path, meta_path) = fixture.cache_paths()?;
    check!(!body_path.exists());
    check!(!meta_path.exists());
    let keys = fixture
        .store
        .journal_keys(census_domain::model::SOURCE_ACQUISITION_PHASE)?;
    check!(eq; keys.iter().filter(|key| key.ends_with("/terminal")).count(), 1);
    Ok(())
}

#[tokio::test]
async fn archive_lock_order_busy_prepared_recovery_keeps_original_unfinished_facts() -> TestResult {
    let fixture = interrupted_fixture(None, 200, CaptureInterruption::AfterArchive).await?;
    let (_, meta_path) = fixture.cache_paths()?;
    let lock_path = meta_path.with_extension("json.lock");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    lock.lock()?;
    let before = fixture
        .store
        .journal_payloads(census_domain::model::SOURCE_ACQUISITION_PHASE)?;
    let result = fixture.acquire(2026, true).await?;
    check!(
        matches!(result, Err(crate::CrawlError::Fetch(FetchError::Cache { path, source }))
        if path == lock_path && source.kind() == ErrorKind::WouldBlock)
    );
    check!(eq; fixture.store.journal_payloads(census_domain::model::SOURCE_ACQUISITION_PHASE)?, before);
    check!(eq; fixture.count(), 1);
    archive_lock_test::require_available(
        &fixture.store.http_cache_dir().join("archive-capacity.lock"),
    )?;
    drop(lock);
    fixture.acquire(2026, true).await??;
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn archive_lock_order_publisher_uses_one_guard_and_preserves_busy_precedence() -> TestResult {
    let fixture = interrupted_fixture(None, 200, CaptureInterruption::AfterArchive).await?;
    fixture.acquire(2026, true).await??;
    let (body_path, meta_path) = fixture.cache_paths()?;
    let before = std::fs::read(&meta_path)?;
    let envelope: EligibleCacheV2 = serde_json::from_slice(&before)?;
    let root = fixture.store.http_cache_dir();
    let archived_body = root
        .join("archive/bodies")
        .join(format!("{}.body", content_digest(BODY)));
    let events = Rc::new(RefCell::new(Vec::new()));
    let observation = observe_recovery(&meta_path, Rc::clone(&events))?;
    crate::net::cache::publish_eligible(&body_path, &meta_path, &envelope, &archived_body)?;
    drop(observation);
    check!(eq; *events.borrow(), vec![LockEvent::BeforeAcquire, LockEvent::Acquired, LockEvent::EligibleAcquired]);
    check!(eq; std::fs::read(&meta_path)?, before);
    let record =
        root.join("archive/reservations")
            .join(format!("{}-{}", "a".repeat(64), "b".repeat(64)));
    std::fs::write(&record, b"malformed")?;
    let lock_path = meta_path.with_extension("json.lock");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    lock.lock()?;
    let result =
        crate::net::cache::publish_eligible(&body_path, &meta_path, &envelope, &archived_body);
    check!(matches!(result, Err(FetchError::Cache { path, source })
        if path == lock_path && source.kind() == ErrorKind::WouldBlock));
    drop(lock);
    let result =
        crate::net::cache::publish_eligible(&body_path, &meta_path, &envelope, &archived_body);
    check!(matches!(result, Err(FetchError::Cache { path, source })
        if path == record && source.kind() == ErrorKind::InvalidData));
    check!(eq; std::fs::read(&meta_path)?, before);
    check!(eq; std::fs::read(&body_path)?, BODY);
    check!(eq; std::fs::read(&record)?, b"malformed");
    archive_lock_test::require_available(&root.join("archive-capacity.lock"))?;
    archive_lock_test::require_available(&lock_path)?;
    Ok(())
}

#[tokio::test]
async fn archive_lock_order_publisher_encoding_and_link_errors_release_guards() -> TestResult {
    let fixture = interrupted_fixture(None, 200, CaptureInterruption::AfterArchive).await?;
    fixture.acquire(2026, true).await??;
    let (body_path, meta_path) = fixture.cache_paths()?;
    let before = std::fs::read(&meta_path)?;
    let envelope: EligibleCacheV2 = serde_json::from_slice(&before)?;
    let root = fixture.store.http_cache_dir();
    let mut invalid = envelope.clone();
    invalid.metadata.content_digest = "invalid".into();
    let result = crate::net::cache::publish_eligible(&body_path, &meta_path, &invalid, &body_path);
    check!(matches!(result, Err(FetchError::Cache { path, source })
        if path == meta_path && source.kind() == ErrorKind::InvalidData));
    archive_lock_test::require_available(&root.join("archive-capacity.lock"))?;
    archive_lock_test::require_available(&meta_path.with_extension("json.lock"))?;
    let missing = root.join("unpublished.body");
    let result = crate::net::cache::publish_eligible(&body_path, &meta_path, &envelope, &missing);
    check!(matches!(result, Err(FetchError::Cache { path, source })
        if path.file_name().is_some_and(|name| name == "cache.body") && source.kind() == ErrorKind::NotFound));
    check!(eq; std::fs::read(&meta_path)?, before);
    check!(eq; std::fs::read(&body_path)?, BODY);
    archive_lock_test::require_available(&root.join("archive-capacity.lock"))?;
    archive_lock_test::require_available(&meta_path.with_extension("json.lock"))?;
    Ok(())
}

#[tokio::test]
async fn archive_lock_order_missing_prepared_invalid_capacity_child() -> TestResult {
    if std::env::var_os("CENSUS_ARCHIVE_LOCK_ORDER_CHILD").is_none() {
        return Ok(());
    }
    let fixture = interrupted_fixture(None, 200, CaptureInterruption::AfterPrepared).await?;
    let before = fixture
        .store
        .journal_payloads(census_domain::model::SOURCE_ACQUISITION_PHASE)?;
    let result = fixture.acquire(2026, true).await?;
    check!(
        matches!(result, Err(crate::CrawlError::Fetch(FetchError::CaptureStore(error)))
        if matches!(error.as_ref(), census_store::StoreError::Capture(census_store::CaptureError::RequiredBackingMissing)))
    );
    check!(eq; fixture.store.journal_payloads(census_domain::model::SOURCE_ACQUISITION_PHASE)?, before);
    check!(eq; fixture.count(), 1);
    check!(!fixture.store.http_cache_dir().join("archive").exists());
    Ok(())
}

#[test]
fn archive_lock_order_missing_prepared_defers_invalid_capacity_in_isolated_process() -> TestResult {
    let output = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "net::acquisition::tests::lock_order::archive_lock_order_missing_prepared_invalid_capacity_child", "--nocapture"])
        .env("CENSUS_ARCHIVE_LOCK_ORDER_CHILD", "1")
        .env("CENSUS_ARCHIVE_CAPACITY_BYTES", "invalid")
        .output()?;
    check!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    check!(std::str::from_utf8(&output.stdout)?.contains("running 1 test"));
    Ok(())
}
