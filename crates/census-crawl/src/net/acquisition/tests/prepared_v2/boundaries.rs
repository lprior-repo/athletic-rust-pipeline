use super::*;
use census_store::{CaptureError, StoreError};

#[tokio::test]
async fn archive_directory_cap_refuses_shared_walk_before_touching_the_old_eligible_pair(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    let (archived_body, _) = publish(&fixture, &prepared)?;
    let directory = archived_body.parent().ok_or("body directory")?;
    for ordinal in 0..4097 {
        std::fs::hard_link(
            &archived_body,
            directory.join(format!("debris-{ordinal:04}")),
        )?;
    }
    let (body, metadata) = fixture.cache_paths()?;
    std::fs::write(&body, b"old body")?;
    std::fs::write(&metadata, b"old metadata")?;
    let mut budget = prepared.budget()?;
    let outcome = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    check!(
        matches!(outcome, PreparedRecoveryOutcome::Retained { eligible: EligiblePublicationOutcome::RefusedBeforeReplace(crate::net::FetchError::CaptureStore(error)), .. } if matches!(*error, StoreError::Capture(CaptureError::ReadBudgetLimitExceeded { resource: "directory events", .. })))
    );
    check!(eq; std::fs::read(body)?, b"old body");
    check!(eq; std::fs::read(metadata)?, b"old metadata");
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test]
async fn invalid_reservation_cannot_delete_an_earlier_validated_record_or_replace_cache(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let reservations = fixture.store.http_cache_dir().join("archive/reservations");
    std::fs::create_dir_all(&reservations)?;
    let valid = reservations.join(format!("{}-{}", "a".repeat(64), "b".repeat(64)));
    let invalid = reservations.join(format!("{}-{}", "c".repeat(64), "d".repeat(64)));
    std::fs::write(&valid, b"0\n0")?;
    std::fs::write(&invalid, b"not\ncounts")?;
    let (body, metadata) = fixture.cache_paths()?;
    std::fs::write(&body, b"old body")?;
    std::fs::write(&metadata, b"old metadata")?;
    let mut budget = prepared.budget()?;
    let outcome = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    check!(
        matches!(outcome, PreparedRecoveryOutcome::Retained { eligible: EligiblePublicationOutcome::RefusedBeforeReplace(crate::net::FetchError::Cache { source, .. }), .. } if source.kind() == std::io::ErrorKind::InvalidData)
    );
    check!(eq; std::fs::read(valid)?, b"0\n0");
    check!(eq; std::fs::read(invalid)?, b"not\ncounts");
    check!(eq; std::fs::read(body)?, b"old body");
    check!(eq; std::fs::read(metadata)?, b"old metadata");
    Ok(())
}

#[tokio::test]
async fn busy_actual_eligible_file_reports_refusal_after_commit_without_touching_old_pair(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let (body, metadata) = fixture.cache_paths()?;
    std::fs::write(&body, b"old body")?;
    std::fs::write(&metadata, b"old metadata")?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(metadata.with_extension("json.lock"))?;
    lock.try_lock()?;
    let mut budget = prepared.budget()?;
    let outcome = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    check!(
        matches!(outcome, PreparedRecoveryOutcome::Retained { eligible: EligiblePublicationOutcome::RefusedBeforeReplace(crate::net::FetchError::Cache { source, .. }), .. } if source.kind() == std::io::ErrorKind::WouldBlock)
    );
    check!(eq; std::fs::read(body)?, b"old body");
    check!(eq; std::fs::read(metadata)?, b"old metadata");
    Ok(())
}

#[tokio::test]
async fn v2_http_root_alias_is_refused_before_recovery_effects() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let root = fixture.store.http_cache_dir();
    let moved = root.with_extension("moved");
    std::fs::rename(&root, &moved)?;
    std::os::unix::fs::symlink(&moved, &root)?;
    let before = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    let mut budget = prepared.budget()?;
    check!(fixture
        .fetcher
        .recover_prepared_target(
            &fixture.store,
            &prepared.owner,
            &prepared.target,
            &mut budget,
            ExactBodySource::NoneSupplied
        )
        .is_err());
    check!(root.symlink_metadata()?.file_type().is_symlink());
    check!(eq; fixture.store.journal_keys(CaptureProtocolVersion::V2.phase())?, before);
    check!(eq; fixture.count(), 0);
    Ok(())
}
