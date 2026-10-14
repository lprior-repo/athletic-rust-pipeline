use super::*;
use census_store::{CaptureError, StoreError};

#[tokio::test]
async fn fixed_missing_closure_is_recorded_once_and_supplied_replay_cannot_create_successor(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    let mut budget = prepared.budget()?;
    let first = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    check!(matches!(
        first,
        PreparedRecoveryOutcome::RetentionFailed { .. }
    ));
    let before = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    let candidate = fixture.root.path().join("exact.body");
    std::fs::write(&candidate, BODY)?;
    let mut budget = prepared.budget()?;
    let repeated = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::SuppliedLocalCopy(&candidate),
    )?;
    check!(matches!(
        repeated,
        PreparedRecoveryOutcome::RetentionFailed { .. }
    ));
    check!(eq; fixture.store.journal_keys(CaptureProtocolVersion::V2.phase())?, before);
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test]
async fn recorded_retention_with_lost_backing_reports_unavailable_without_successor() -> TestResult
{
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    let (body, _) = publish(&fixture, &prepared)?;
    let mut budget = prepared.budget()?;
    check!(matches!(
        fixture.fetcher.recover_prepared_target(
            &fixture.store,
            &prepared.owner,
            &prepared.target,
            &mut budget,
            ExactBodySource::NoneSupplied
        )?,
        PreparedRecoveryOutcome::Retained { .. }
    ));
    let before = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    std::fs::remove_file(body)?;
    let mut budget = prepared.budget()?;
    let outcome = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    let PreparedRecoveryOutcome::RetainedUnavailable {
        occurrence,
        terminal,
        receipt,
        ..
    } = outcome
    else {
        return Err("recorded retained unavailable".into());
    };
    check!(eq; occurrence.attempt(), prepared.target.attempt());
    check!(eq; terminal.attempt(), prepared.target.attempt());
    check!(eq; fixture.store.receipt(&receipt.operation)?, Some(receipt.clone()));
    check!(eq; receipt.digest, serialized_digest(&terminal)?);
    check!(eq; fixture.store.journal_keys(CaptureProtocolVersion::V2.phase())?, before);
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test]
async fn occupied_corrupt_immutable_body_is_repair_required_and_never_quarantined_or_rewritten(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    let (body, metadata) = publish(&fixture, &prepared)?;
    let mut corrupt = BODY.to_vec();
    let first = corrupt.first_mut().ok_or("nonempty fixture")?;
    *first ^= 1;
    std::fs::write(&body, &corrupt)?;
    let before = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    let mut budget = prepared.budget()?;
    let result = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    );
    check!(
        matches!(result, Err(crate::net::FetchError::CaptureStore(error)) if matches!(*error, StoreError::Capture(CaptureError::PreparedRepairRequired)))
    );
    check!(eq; std::fs::read(body)?, corrupt);
    check!(eq; std::fs::read(metadata)?, prepared.metadata);
    check!(!fixture.store.http_cache_dir().join("quarantine").exists());
    check!(eq; fixture.store.journal_keys(CaptureProtocolVersion::V2.phase())?, before);
    Ok(())
}

#[tokio::test]
async fn supplied_exact_and_empty_bodies_are_real_local_restorations() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    for bytes in [BODY, b"".as_slice()] {
        let fixture = Fixture::new_v2(&listener)?;
        let prepared = prepare(&fixture, bytes, Encoding::Compact)?;
        let candidate = fixture.root.path().join("exact.body");
        std::fs::write(&candidate, bytes)?;
        let mut budget = prepared.budget()?;
        let outcome = fixture.fetcher.recover_prepared_target(
            &fixture.store,
            &prepared.owner,
            &prepared.target,
            &mut budget,
            ExactBodySource::SuppliedLocalCopy(&candidate),
        )?;
        let PreparedRecoveryOutcome::Retained {
            captured, eligible, ..
        } = outcome
        else {
            return Err("retained supplied body".into());
        };
        check!(eq; captured.capture().body, bytes);
        check!(matches!(
            eligible,
            EligiblePublicationOutcome::Skipped(EligiblePublicationSkip::NonRoundtrippingMetadata)
        ));
        check!(eq; std::fs::read(&candidate)?, bytes);
        check!(eq; fixture.count(), 0);
    }
    Ok(())
}

#[tokio::test]
async fn recovery_context_rejects_foreign_work_and_same_target_wrong_meet_without_consuming(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let mut budget = prepared.budget()?;
    let PreparedRecoveryOutcome::Retained { captured, .. } =
        fixture.fetcher.recover_prepared_target(
            &fixture.store,
            &prepared.owner,
            &prepared.target,
            &mut budget,
            ExactBodySource::NoneSupplied,
        )?
    else {
        return Err("retained".into());
    };
    let reference = crate::milesplit::ResultSetRef::parse(
        "https://mi.milesplit.com/meets/999999/results/1266814/raw",
    )
    .ok_or("foreign reference")?;
    let request = crate::milesplit::owned_meet_request(
        &fixture.fetcher,
        &reference,
        &crate::net::FetchOptions::default(),
    )?;
    let foreign = prepared.owner.for_owned_meet(
        census_domain::UsJurisdiction::Michigan,
        "999999",
        request,
    )?;
    check!(crate::ResultsCaptureContext::from_prepared_recovery(
        prepared.owner.clone(),
        prepared.target.clone(),
        foreign,
        captured
    )
    .is_err());
    let mut budget = prepared.budget()?;
    let PreparedRecoveryOutcome::Retained { captured, .. } =
        fixture.fetcher.recover_prepared_target(
            &fixture.store,
            &prepared.owner,
            &prepared.target,
            &mut budget,
            ExactBodySource::NoneSupplied,
        )?
    else {
        return Err("retained repeat".into());
    };
    let context = crate::ResultsCaptureContext::from_prepared_recovery(
        prepared.owner,
        prepared.target,
        prepared.work,
        captured,
    )?;
    check!(context
        .take_prepared_recovery(census_domain::UsJurisdiction::Michigan, "999999")
        .is_err());
    check!(context
        .take_prepared_recovery(census_domain::UsJurisdiction::Michigan, "725218")?
        .is_some());
    Ok(())
}

#[tokio::test]
async fn ordinary_v2_rerun_refuses_unplanned_prepared_without_recovery_or_source_request(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    let before = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    let result = fixture.acquire(2026, false).await?;
    check!(
        matches!(result, Err(crate::CrawlError::Fetch(crate::net::FetchError::CaptureStore(error))) if matches!(*error, StoreError::Capture(CaptureError::PreparedTargetMismatch)))
    );
    check!(eq; fixture.store.journal_keys(CaptureProtocolVersion::V2.phase())?, before);
    check!(eq; fixture.count(), 0);
    let mut budget = prepared.budget()?;
    check!(matches!(
        fixture.fetcher.recover_prepared_target(
            &fixture.store,
            &prepared.owner,
            &prepared.target,
            &mut budget,
            ExactBodySource::NoneSupplied
        )?,
        PreparedRecoveryOutcome::RetentionFailed { .. }
    ));
    Ok(())
}

#[tokio::test]
async fn thirteen_target_preflight_refuses_before_archive_effects() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    let mut selection = AcquisitionReadBudget::for_selection();
    for _ in 0..13 {
        check!(matches!(
            fixture.store.read_existing_recovery_target(
                &prepared.owner,
                &prepared.work,
                &mut selection
            )?,
            ExistingRecoveryRoute::Prepared(_)
        ));
    }
    check!(matches!(
        PreparedRecoveryBudget::admit(selection.selection_usage(), 13),
        Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
            resource: "backing bytes"
        }))
    ));
    check!(!fixture
        .store
        .http_cache_dir()
        .join("archive/bodies")
        .exists());
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test]
async fn both_held_phases_use_the_same_pre_admitted_capacity_scalar() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    let capacity = u64::try_from(BODY.len())?
        .checked_add(u64::try_from(prepared.metadata.len())?)
        .ok_or("capacity overflow")?;
    let candidate = fixture.root.path().join("exact.body");
    std::fs::write(&candidate, BODY)?;
    let mut budget = prepared.budget()?;
    let read = fixture
        .store
        .begin_prepared_recovery(&prepared.owner, &prepared.target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(session) = read else {
        return Err("missing Prepared".into());
    };
    let outcome = super::super::super::recovery::resolve_unresolved(
        session,
        ExactBodySource::SuppliedLocalCopy(&candidate),
        capacity,
    )?;
    let PreparedRecoveryOutcome::Retained {
        captured, eligible, ..
    } = outcome
    else {
        return Err("retained restoration".into());
    };
    check!(eq; captured.capture().body, BODY);
    check!(
        matches!(eligible, EligiblePublicationOutcome::RefusedBeforeReplace(crate::net::FetchError::ArchiveCapacity { capacity: observed, used, requested, .. }) if observed == capacity && used == capacity && requested > 0)
    );
    let (body, metadata) = fixture.cache_paths()?;
    check!(!body.exists());
    check!(!metadata.exists());
    check!(eq; fixture.count(), 0);
    Ok(())
}
