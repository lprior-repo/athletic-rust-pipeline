use super::*;
use crate::net::cache::archive_lock_test::{self, LockEvent};
use std::cell::RefCell;
use std::rc::Rc;

#[tokio::test]
async fn compatibility_skips_leave_busy_old_pair_untouched_before_any_eligible_event() -> TestResult
{
    for encoding in [Encoding::Compact, Encoding::Unknown] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let fixture = Fixture::new_v2(&listener)?;
        let prepared = prepare(&fixture, BODY, encoding)?;
        publish(&fixture, &prepared)?;
        let (body, metadata) = fixture.cache_paths()?;
        std::fs::write(&body, b"previous body")?;
        std::fs::write(&metadata, b"previous metadata")?;
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(metadata.with_extension("json.lock"))?;
        lock.try_lock()?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = Rc::clone(&events);
        let _observer = archive_lock_test::observe(move |_, event| {
            observed.borrow_mut().push(event);
            Ok(())
        })?;
        let mut budget = prepared.budget()?;
        let outcome = fixture.fetcher.recover_prepared_target(
            &fixture.store,
            &prepared.owner,
            &prepared.target,
            &mut budget,
            ExactBodySource::NoneSupplied,
        )?;
        check!(matches!(
            outcome,
            PreparedRecoveryOutcome::Retained {
                eligible: EligiblePublicationOutcome::Skipped(
                    EligiblePublicationSkip::NonRoundtrippingMetadata
                ),
                ..
            }
        ));
        check!(events.borrow().is_empty());
        check!(eq; std::fs::read(body)?, b"previous body");
        check!(eq; std::fs::read(metadata)?, b"previous metadata");
        check!(eq; fixture.count(), 0);
    }
    Ok(())
}

#[tokio::test]
async fn canonical_publication_streams_and_preserves_unknown_historical_metadata_identity(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let (body, metadata) = fixture.cache_paths()?;
    let mut value: serde_json::Value = serde_json::from_slice(&prepared.metadata)?;
    value.as_object_mut().ok_or("metadata object")?.insert(
        "unknown".into(),
        serde_json::json!({"z":[null,{"b":2,"a":1}],"a":true}),
    );
    value
        .as_object_mut()
        .ok_or("metadata object")?
        .insert("etag".into(), serde_json::Value::Null);
    let raw = serde_json::to_vec(&value)?;
    let old = crate::net::cache::Capture::from_bytes(&raw, &metadata)?;
    std::fs::write(&body, BODY)?;
    std::fs::write(&metadata, &raw)?;
    let mut budget = prepared.budget()?;
    let outcome = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    check!(matches!(
        outcome,
        PreparedRecoveryOutcome::Retained {
            eligible: EligiblePublicationOutcome::Published,
            ..
        }
    ));
    let digest = crate::net::cache::content_digest(&old.encoded);
    let archive = fixture
        .store
        .http_cache_dir()
        .join("archive/captures")
        .join(&old.meta.content_digest)
        .join(format!("{digest}.meta.json"));
    check!(eq; std::fs::read(archive)?, old.encoded);
    let current: serde_json::Value = serde_json::from_slice(&std::fs::read(metadata)?)?;
    check!(eq; current.get("version").and_then(serde_json::Value::as_u64), Some(2));
    check!(eq; std::fs::read(body)?, BODY);
    Ok(())
}

#[tokio::test]
async fn admitted_publication_holds_actual_same_archive_file_until_after_eligible() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let (_, metadata) = fixture.cache_paths()?;
    let eligible = metadata.with_extension("json.lock");
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&events);
    let _observer = archive_lock_test::observe(move |archive, event| {
        if event == LockEvent::EligibleAcquired {
            archive_lock_test::require_held(archive)?;
            archive_lock_test::require_held(&eligible)?;
        }
        observed.borrow_mut().push(event);
        Ok(())
    })?;
    let mut budget = prepared.budget()?;
    let outcome = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    check!(matches!(
        outcome,
        PreparedRecoveryOutcome::Retained {
            eligible: EligiblePublicationOutcome::Published,
            ..
        }
    ));
    check!(eq; &*events.borrow(), &[LockEvent::EligibleAcquired]);
    archive_lock_test::require_available(
        &fixture.store.http_cache_dir().join("archive-capacity.lock"),
    )?;
    Ok(())
}

#[tokio::test]
async fn second_rename_failure_keeps_committed_truth_and_reports_actual_partial_replacement(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let (body, metadata) = fixture.cache_paths()?;
    let target = metadata.clone();
    let _fault = crate::net::cache::recovery::replace::testing::after_body(move || {
        std::fs::create_dir(&target)?;
        Ok(())
    })?;
    let mut budget = prepared.budget()?;
    let outcome = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    let PreparedRecoveryOutcome::Retained {
        captured,
        terminal,
        receipt,
        eligible,
    } = outcome
    else {
        return Err("retained after replacement failure".into());
    };
    check!(matches!(
        eligible,
        EligiblePublicationOutcome::FailedDuringReplace(crate::net::FetchError::Cache { .. })
    ));
    check!(eq; std::fs::read(body)?, BODY);
    check!(metadata.is_dir());
    check!(eq; captured.capture().body, BODY);
    check!(eq; terminal.attempt(), prepared.target.attempt());
    check!(eq; fixture.store.receipt(&receipt.operation)?, Some(receipt.clone()));
    check!(eq; receipt.digest, serialized_digest(&terminal)?);
    check!(eq; fixture.count(), 0);
    Ok(())
}
