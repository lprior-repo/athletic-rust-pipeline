use super::*;
use crate::net::cache::archive_lock_test::{self, LockEvent};
use census_store::{CaptureError, StoreError};
use std::cell::RefCell;
use std::rc::Rc;

#[tokio::test(flavor = "current_thread")]
async fn fixed_v2_same_content_successor_keeps_new_occurrence_and_original_loss_receipt(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let original = prepare(&fixture, BODY, Encoding::Canonical)?;
    let mut budget = original.budget()?;
    let first = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &original.owner,
        &original.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    let PreparedRecoveryOutcome::RetentionFailed {
        terminal: failed,
        receipt: loss,
    } = first
    else {
        return Err("original loss closure".into());
    };
    check!(eq; failed.attempt(), original.target.attempt());
    check!(eq; loss.digest, serialized_digest(&failed)?);
    check!(eq; fixture.store.receipt(&loss.operation)?, Some(loss.clone()));
    let successor = prepare(&fixture, BODY, Encoding::Canonical)?;
    check!(eq; successor.target.attempt().ordinal().get(), 2);
    check!(eq; successor.target.attempt().work_id(), original.target.attempt().work_id());
    check!(successor.target.prepared_digest() != original.target.prepared_digest());
    check!(eq; successor.metadata, original.metadata);
    publish(&fixture, &successor)?;
    let expected = successor.target.attempt().clone();
    let mut budget = successor.budget()?;
    let recovered = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &successor.owner,
        &successor.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    let PreparedRecoveryOutcome::Retained {
        captured,
        terminal,
        receipt,
        eligible,
    } = recovered
    else {
        return Err("retained successor".into());
    };
    check!(matches!(eligible, EligiblePublicationOutcome::Published));
    check!(eq; terminal.attempt(), &expected);
    check!(eq; receipt.digest, serialized_digest(&terminal)?);
    check!(eq; fixture.store.receipt(&receipt.operation)?, Some(receipt.clone()));
    check!(eq; captured.capture().fetched_at, CAPTURED);
    check!(eq; captured.capture().body, BODY);
    let pointer = captured.capture().body.as_ptr();
    let context = crate::ResultsCaptureContext::from_prepared_recovery(
        successor.owner,
        successor.target,
        successor.work,
        captured,
    )?;
    let ctx = fixture.context(&context, false)?;
    let interpreted =
        crate::milesplit::fetch_owned_meet(&ctx, &fixture.reference()?, &ctx.fetch_options())
            .await?;
    check!(matches!(
        interpreted.verdict(),
        crate::milesplit::OwnedMeetVerdict::Parsed(_)
    ));
    check!(eq; Fixture::occurrence(&interpreted)?.attempt(), &expected);
    check!(Fixture::occurrence(&interpreted)?.attempt() != original.target.attempt());
    check!(eq; interpreted.capture().body.as_ptr(), pointer);
    let before = fixture
        .store
        .journal_payloads(CaptureProtocolVersion::V2.phase())?;
    let mut budget = original.budget()?;
    let repeated = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &original.owner,
        &original.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    let PreparedRecoveryOutcome::RetentionFailed { terminal, receipt } = repeated else {
        return Err("fixed original loss replay".into());
    };
    check!(eq; terminal, failed);
    check!(eq; receipt, loss);
    check!(eq; fixture.store.receipt(&loss.operation)?, Some(loss.clone()));
    check!(eq; fixture.store.journal_payloads(CaptureProtocolVersion::V2.phase())?, before);
    let keys = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    check!(eq; keys.iter().filter(|key| key.ends_with("/terminal")).count(), 2);
    check!(!keys.iter().any(|key| key.contains("/attempt/3/")));
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn admitted_canonical_publication_runs_once_per_committed_loan_and_denies_second_issue(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let mut budget = prepared.budget()?;
    let read = fixture
        .store
        .begin_prepared_recovery(&prepared.owner, &prepared.target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(session) = read else {
        return Err("exact Prepared session".into());
    };
    let mut retained = session.finish()?;
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&events);
    let _observer = archive_lock_test::observe(move |_, event| {
        observed.borrow_mut().push(event);
        Ok(())
    })?;
    {
        let (fence, input, occurrence, io) = retained.publication_io()?;
        let outcome = crate::net::cache::recovery::publish_recovered_eligible_held(
            fence,
            input,
            occurrence,
            u64::MAX,
            io,
        );
        check!(matches!(outcome, EligiblePublicationOutcome::Published));
    }
    let (body, metadata) = fixture.cache_paths()?;
    let body_before = std::fs::read(&body)?;
    let metadata_before = std::fs::read(&metadata)?;
    check!(eq; body_before, BODY);
    let encoded: serde_json::Value = serde_json::from_slice(&metadata_before)?;
    check!(eq; encoded.get("version").and_then(serde_json::Value::as_u64), Some(2));
    check!(eq; &*events.borrow(), &[LockEvent::EligibleAcquired]);
    check!(matches!(
        retained.publication_io(),
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
    check!(eq; std::fs::read(&body)?, body_before);
    check!(eq; std::fs::read(&metadata)?, metadata_before);
    check!(eq; &*events.borrow(), &[LockEvent::EligibleAcquired]);
    let parts = retained.into_parts();
    check!(eq; parts.body, BODY);
    check!(eq; parts.prepared.metadata_bytes(), prepared.metadata);
    check!(eq; parts.terminal.attempt(), prepared.target.attempt());
    check!(eq; parts.receipt.digest, serialized_digest(&parts.terminal)?);
    check!(eq; fixture.store.receipt(&parts.receipt.operation)?, Some(parts.receipt));
    archive_lock_test::require_available(
        &fixture.store.http_cache_dir().join("archive-capacity.lock"),
    )?;
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn actual_held_restoration_preserves_occupied_corrupt_metadata_and_store_refuses_commit(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    let (body, metadata) = publish(&fixture, &prepared)?;
    std::fs::remove_file(&body)?;
    let candidate = fixture.root.path().join("exact.body");
    std::fs::write(&candidate, BODY)?;
    let before = fixture
        .store
        .journal_payloads(CaptureProtocolVersion::V2.phase())?;
    let mut budget = prepared.budget()?;
    let receipts_before = fixture.store.receipt_count()?;
    let read = fixture
        .store
        .begin_prepared_recovery(&prepared.owner, &prepared.target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(mut session) = read else {
        return Err("missing body restoration".into());
    };
    session.prepare_restore(ExactBodySource::SuppliedLocalCopy(&candidate))?;
    let mut corrupt = prepared.metadata.clone();
    let first = corrupt.first_mut().ok_or("nonempty metadata")?;
    *first ^= 1;
    {
        let (fence, input, held, io) = session.restoration_io()?;
        std::fs::write(&metadata, &corrupt)?;
        crate::net::cache::recovery::restore_exact_prepared_held(fence, input, held, u64::MAX, io)?;
    }
    check!(eq; std::fs::read(&body)?, BODY);
    check!(eq; std::fs::read(&metadata)?, corrupt);
    check!(matches!(
        session.verify_restored(),
        Err(StoreError::Capture(CaptureError::PreparedRepairRequired))
    ));
    check!(matches!(
        session.finish(),
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
    check!(eq; fixture.store.journal_payloads(CaptureProtocolVersion::V2.phase())?, before);
    check!(eq; fixture.store.receipt_count()?, receipts_before);
    check!(!fixture.store.http_cache_dir().join("quarantine").exists());
    let (eligible_body, eligible_metadata) = fixture.cache_paths()?;
    check!(!eligible_body.exists());
    check!(!eligible_metadata.exists());
    check!(eq; std::fs::read(&metadata)?, corrupt);
    check!(eq; std::fs::read(&candidate)?, BODY);
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test]
async fn retained_consumer_allocation_refusal_preserves_committed_parts() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let mut budget = prepared.budget()?;
    let read = fixture
        .store
        .begin_prepared_recovery(&prepared.owner, &prepared.target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(session) = read else {
        return Err("exact Prepared session".into());
    };
    let mut retained = session.finish()?;
    check!(matches!(
        retained.reserve_consumer_owned(usize::MAX),
        Err(StoreError::Capture(CaptureError::ReadBudgetExceeded { .. }))
            | Err(StoreError::CounterOverflow)
    ));
    retained.reserve_consumer_owned(std::mem::size_of::<crate::net::CapturedFetch>())?;
    let parts = retained.into_parts();
    check!(eq; parts.body, BODY);
    check!(eq; parts.terminal.attempt(), prepared.target.attempt());
    check!(eq; parts.receipt.digest, serialized_digest(&parts.terminal)?);
    check!(eq; fixture.store.receipt(&parts.receipt.operation)?, Some(parts.receipt));
    let (body, metadata) = fixture.cache_paths()?;
    check!(!body.exists());
    check!(!metadata.exists());
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[test]
fn prepared_recovery_outcome_stays_within_512_bytes() -> TestResult {
    check!(std::mem::size_of::<PreparedRecoveryOutcome>() <= 512);
    Ok(())
}
