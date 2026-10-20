use super::*;
use crate::net::cache::recovery::diagnostic_testing::{self, Primary};
use census_store::{CaptureError, StoreError};

#[tokio::test]
async fn actual_shared_io_preserves_typed_original_and_cleanup_errors_within_one_cap() -> TestResult
{
    exercise(1100, false).await
}

#[tokio::test]
async fn actual_shared_io_refuses_combined_original_cleanup_diagnostics_without_reset() -> TestResult
{
    exercise(3900, true).await
}

async fn exercise(name_bytes: usize, exhausted: bool) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let root = fixture.store.http_cache_dir();
    let invalid = root.join("x".repeat(name_bytes));
    let mut budget = prepared.budget()?;
    let read = fixture
        .store
        .begin_prepared_recovery(&prepared.owner, &prepared.target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(session) = read else {
        return Err("exact Prepared session".into());
    };
    let mut retained = session.finish()?;
    let before = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    {
        let (_fence, _prepared, _occurrence, mut io) = retained.publication_io()?;
        let result = crate::net::cache::recovery::diagnostic_testing::actual_original_and_cleanup(
            &invalid,
            &root,
            Primary::Create,
            &mut io,
        );
        let Err(error) = result else {
            return Err("actual overlong-path filesystem errors".into());
        };
        if exhausted {
            check!(crate::net::cache::recovery::diagnostic_testing::retains_primary_and_diagnostic_refusal(&error));
        } else {
            check!(
                crate::net::cache::recovery::diagnostic_testing::retains_both_typed_causes(&error)
            );
        }
    }
    let parts = retained.into_parts();
    check!(eq; parts.body, BODY);
    check!(eq; parts.terminal.attempt(), prepared.target.attempt());
    check!(eq; fixture.store.journal_keys(CaptureProtocolVersion::V2.phase())?, before);
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test]
async fn compound_store_budget_primary_keeps_exact_resource_and_shared_classification() -> TestResult
{
    classify(Primary::StoreBudget).await
}

#[tokio::test]
async fn compound_cache_would_block_primary_keeps_kind_and_shared_classification() -> TestResult {
    classify(Primary::WouldBlock).await
}

#[tokio::test]
async fn compound_cache_invalid_data_primary_keeps_kind_and_shared_classification() -> TestResult {
    classify(Primary::InvalidData).await
}

async fn classify(kind: Primary) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let root = fixture.store.http_cache_dir();
    let invalid = root.join("x".repeat(300));
    let (_, metadata) = fixture.cache_paths()?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(metadata.with_extension("json.lock"))?;
    lock.try_lock()?;
    let cache_kind = match kind {
        Primary::WouldBlock => Some(std::io::ErrorKind::WouldBlock),
        Primary::InvalidData => Some(std::io::ErrorKind::InvalidData),
        _ => None,
    };
    let anchor = if cache_kind == Some(std::io::ErrorKind::WouldBlock) {
        &metadata
    } else {
        &root
    };
    let mut budget = prepared.budget()?;
    let read = fixture
        .store
        .begin_prepared_recovery(&prepared.owner, &prepared.target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(session) = read else {
        return Err("exact Prepared session".into());
    };
    let mut retained = session.finish()?;
    {
        let (_fence, _prepared, _occurrence, mut io) = retained.publication_io()?;
        let result =
            diagnostic_testing::actual_original_and_cleanup(&invalid, anchor, kind, &mut io);
        let Err(error) = result else {
            return Err("compound failure".into());
        };
        check!(diagnostic_testing::retains_both_typed_causes(&error));
        check!(!error.retryable());
        check!(!error.halts_object_walk());
        if let Some(kind) = cache_kind {
            check!(
                matches!(&error, crate::net::FetchError::Cache { source, .. } if source.kind() == kind)
            );
            check!(eq; super::super::super::observation::failure_of(&error), AcquisitionFailureV1::ArchiveIntegrity);
        } else {
            check!(
                matches!(error.recovery_primary(), Some(crate::net::RecoveryFailurePrimary::Store(error)) if matches!(**error, StoreError::Capture(CaptureError::ReadBudgetLimitExceeded { resource: "backing bytes", .. })))
            );
            check!(eq; super::super::super::observation::failure_of(&error), AcquisitionFailureV1::AdmissionRefused);
        }
    }
    check!(eq; retained.into_parts().body, BODY);
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test]
async fn third_cleanup_leaf_refuses_before_another_compound_allocation() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let root = fixture.store.http_cache_dir();
    let invalid = root.join("x".repeat(300));
    let mut budget = prepared.budget()?;
    let read = fixture
        .store
        .begin_prepared_recovery(&prepared.owner, &prepared.target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(session) = read else {
        return Err("exact Prepared session".into());
    };
    let mut retained = session.finish()?;
    {
        let (_fence, _prepared, _occurrence, mut io) = retained.publication_io()?;
        let result = diagnostic_testing::third_cleanup(&invalid, &root, &mut io);
        check!(
            matches!(result, Err(crate::net::FetchError::CaptureStore(error)) if matches!(*error, StoreError::Capture(CaptureError::TransitionRefused)))
        );
    }
    check!(eq; retained.into_parts().body, BODY);
    Ok(())
}

#[tokio::test]
async fn five_box_fallback_tail_uses_one_admission_and_returns_exact_diagnostic_refusal(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let invalid = fixture.store.http_cache_dir().join("x".repeat(300));
    let mut budget = prepared.budget()?;
    let read = fixture
        .store
        .begin_prepared_recovery(&prepared.owner, &prepared.target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(session) = read else {
        return Err("exact Prepared session".into());
    };
    let mut retained = session.finish()?;
    {
        let (_fence, _prepared, _occurrence, mut io) = retained.publication_io()?;
        let result = diagnostic_testing::fallback_store_boxes(&invalid, &mut io);
        check!(
            matches!(result, Err(crate::net::FetchError::CaptureStore(error)) if matches!(*error, StoreError::Capture(CaptureError::ReadBudgetLimitExceeded { resource: "diagnostic bytes", .. })))
        );
    }
    check!(!invalid.exists());
    check!(eq; retained.into_parts().body, BODY);
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test]
async fn publication_error_box_admission_refuses_before_old_pair_or_stage_effects() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let (body, metadata) = fixture.cache_paths()?;
    std::fs::write(&body, b"old body")?;
    std::fs::write(&metadata, b"old metadata")?;
    let root = fixture.store.http_cache_dir();
    let before = std::fs::read_dir(&root)?
        .collect::<Result<Vec<_>, _>>()?
        .len();
    let mut budget = prepared.budget()?;
    let read = fixture
        .store
        .begin_prepared_recovery(&prepared.owner, &prepared.target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(session) = read else {
        return Err("exact Prepared session".into());
    };
    let mut retained = session.finish()?;
    {
        let (fence, input, occurrence, mut io) = retained.publication_io()?;
        diagnostic_testing::deplete_requested_owned(&mut io)?;
        let outcome = crate::net::cache::recovery::publish_recovered_eligible_held(
            fence,
            input,
            occurrence,
            u64::MAX,
            io,
        );
        check!(
            matches!(outcome, EligiblePublicationOutcome::RefusedBeforeReplace(crate::net::FetchError::CaptureStore(error)) if matches!(*error, StoreError::Capture(CaptureError::ReadBudgetLimitExceeded { resource: "allocation", .. })))
        );
    }
    check!(eq; std::fs::read(&body)?, b"old body");
    check!(eq; std::fs::read(&metadata)?, b"old metadata");
    check!(eq; std::fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?.len(), before);
    check!(eq; retained.into_parts().body, BODY);
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test]
async fn actual_error_box_preadmission_rejects_exhausted_requested_account() -> TestResult {
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
    {
        let (_fence, _prepared, _occurrence, mut io) = retained.publication_io()?;
        let result = diagnostic_testing::entry_admission_after_depletion(&mut io);
        check!(
            matches!(result, Err(crate::net::FetchError::CaptureStore(error)) if matches!(*error, StoreError::Capture(CaptureError::ReadBudgetLimitExceeded { resource: "allocation", .. })))
        );
    }
    check!(eq; retained.into_parts().body, BODY);
    check!(eq; fixture.count(), 0);
    Ok(())
}
