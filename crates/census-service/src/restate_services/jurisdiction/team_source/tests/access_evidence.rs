use super::*;
use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use census_store::Table;

fn condition(at: &str, status: u16) -> SourceAccessCondition {
    SourceAccessCondition::new(
        "original-source",
        "example.org",
        AccessBlockKind::RateLimited,
        status,
        at,
        "original cause",
    )
    .with_retry_after(Some(120))
    .with_cooldown_until(Some("2026-10-14T12:00:00Z".into()))
}

#[test]
fn terminal_attempt_commits_exact_access_evidence_and_replay_does_not_invent_rows() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let expected = condition("2026-10-14T11:00:00Z", 429);
    {
        let store = Store::open(root.path())?;
        let (attempt, _) = reserve(&store, &request)?;
        let outcome = ledger::finish(
            &store,
            &key(&request),
            attempt,
            ledger::Attempt::Failed(JobError::Terminal {
                message: "original terminal".into(),
            }),
            std::slice::from_ref(&expected),
        )?;
        check!(
            matches!(outcome, ledger::Completion::Settled(TeamsSourceOutcome::Terminal { message, .. }) if message == "original terminal")
        );
    }
    let store = Store::open(root.path())?;
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![expected.clone()]);
    let foreign = SourceAccessCondition::new(
        "foreign",
        "other.org",
        AccessBlockKind::Forbidden,
        403,
        "2026-10-14T11:05:00Z",
        "must not be manufactured by replay",
    );
    ledger::finish(
        &store,
        &key(&request),
        1,
        ledger::Attempt::Failed(JobError::Transient {
            message: "ignored replay input".into(),
        }),
        &[foreign],
    )?;
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![expected]);
    check!(eq; ledger::durable_attempts(&store)?.reserved, 1);
    Ok(())
}

#[test]
fn transient_attempt_preserves_classification_and_access_while_older_flush_cannot_regress(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let newer = condition("2026-10-14T11:00:00Z", 429);
    let (attempt, _) = reserve(&store, &request)?;
    let outcome = ledger::finish(
        &store,
        &key(&request),
        attempt,
        ledger::Attempt::Failed(JobError::Transient {
            message: "retry original".into(),
        }),
        std::slice::from_ref(&newer),
    )?;
    check!(
        matches!(outcome, ledger::Completion::Retry(JobError::Transient { message }) if message == "retry original")
    );
    let older = condition("2026-10-13T11:00:00Z", 403)
        .with_retry_after(Some(10))
        .with_cooldown_until(Some("2026-10-13T12:00:00Z".into()));
    let (attempt, _) = reserve(&store, &request)?;
    ledger::finish(
        &store,
        &key(&request),
        attempt,
        ledger::Attempt::Failed(JobError::Terminal {
            message: "stop original".into(),
        }),
        &[older],
    )?;
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![newer]);
    check!(eq; ledger::durable_attempts(&store)?.reserved, 2);
    Ok(())
}

#[test]
fn rejected_access_snapshot_cannot_commit_an_attempt_outcome() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let (attempt, _) = reserve(&store, &request)?;
    let mut oversized = condition("2026-10-14T11:00:00Z", 429);
    oversized.detail = "x".repeat(census_store::MAX_OBSERVATION_RECORD_BYTES);
    check!(ledger::finish(
        &store,
        &key(&request),
        attempt,
        ledger::Attempt::Failed(JobError::Terminal {
            message: "uncommitted original".into()
        }),
        &[oversized]
    )
    .is_err());
    check!(store
        .scan::<SourceAccessCondition>(Table::SourceAccess)?
        .is_empty());
    let attempts = ledger::durable_attempts(&store)?;
    check!(eq; (attempts.reserved, attempts.recorded, attempts.unknown), (1, 0, 1));
    let (next, _) = reserve(&store, &request)?;
    check!(eq; next, 2);
    Ok(())
}

#[test]
fn historical_empty_access_outcome_remains_empty_on_replay() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let (attempt, _) = reserve(&store, &request)?;
    ledger::finish(
        &store,
        &key(&request),
        attempt,
        ledger::Attempt::Failed(JobError::Terminal {
            message: "historical refusal".into(),
        }),
        &[],
    )?;
    ledger::finish(
        &store,
        &key(&request),
        attempt,
        ledger::Attempt::Failed(JobError::Terminal {
            message: "new text".into(),
        }),
        &[condition("2026-10-14T11:00:00Z", 429)],
    )?;
    check!(store
        .scan::<SourceAccessCondition>(Table::SourceAccess)?
        .is_empty());
    Ok(())
}

#[test]
fn access_snapshot_failure_preserves_primary_retry_classification() -> TestResult {
    let secondary = || census_crawl::CrawlError::Resource {
        resource: "source access snapshot",
        requested: 4097,
        limit: 4096,
    };
    let transient = super::super::access_snapshot::failure(
        ledger::Attempt::Failed(JobError::Transient {
            message: "HTTP 429 original".into(),
        }),
        secondary(),
    );
    check!(
        matches!(transient, ledger::Attempt::Failed(JobError::Transient { message }) if message.contains("HTTP 429 original") && message.contains("source access snapshot"))
    );
    let terminal = super::super::access_snapshot::failure(
        ledger::Attempt::Failed(JobError::Terminal {
            message: "HTTP 403 original".into(),
        }),
        secondary(),
    );
    check!(
        matches!(terminal, ledger::Attempt::Failed(JobError::Terminal { message }) if message.contains("HTTP 403 original") && message.contains("source access snapshot"))
    );
    Ok(())
}

#[test]
fn access_staging_refusal_keeps_transient_primary_and_bounded_pair() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let (attempt, _) = reserve(&store, &request)?;
    let mut oversized = condition("2026-10-14T11:00:00Z", 429);
    oversized.detail = "x".repeat(census_store::MAX_OBSERVATION_RECORD_BYTES);
    let result = ledger::finish(
        &store,
        &key(&request),
        attempt,
        ledger::Attempt::Failed(JobError::Transient {
            message: format!("HTTP 429 original {}", "z".repeat(9000)),
        }),
        &[oversized],
    );
    check!(matches!(result, Err(JobError::Transient { message })
        if message.contains("HTTP 429 original") && message.contains("retaining access evidence also failed") && message.len() < 9000));
    let attempts = ledger::durable_attempts(&store)?;
    check!(eq; (attempts.reserved, attempts.recorded, attempts.unknown), (1, 0, 1));
    check!(store
        .scan::<SourceAccessCondition>(Table::SourceAccess)?
        .is_empty());
    Ok(())
}

#[test]
fn access_commit_decode_failure_keeps_primary_and_uncommitted_outcome() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let row = condition("2026-10-14T11:00:00Z", 429);
    store.replace_many(
        Table::SourceAccess,
        &[serde_json::json!({"id":row.id,"detail":"missing required fields"})],
    )?;
    let (attempt, _) = reserve(&store, &request)?;
    let generation = store.evidence_generation();
    let result = ledger::finish(
        &store,
        &key(&request),
        attempt,
        ledger::Attempt::Failed(JobError::Transient {
            message: "HTTP 429 original".into(),
        }),
        &[row],
    );
    check!(matches!(result, Err(JobError::Transient { message })
        if message.contains("HTTP 429 original") && message.contains("decoding retained source access condition")));
    check!(eq; store.evidence_generation(), generation);
    let attempts = ledger::durable_attempts(&store)?;
    check!(eq; (attempts.reserved, attempts.recorded, attempts.unknown), (1, 0, 1));
    Ok(())
}

#[test]
fn access_staging_refusal_retains_typed_resource_primary() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request()?;
    let (attempt, _) = reserve(&store, &request)?;
    let mut oversized = condition("2026-10-14T11:00:00Z", 429);
    oversized.detail = "x".repeat(census_store::MAX_OBSERVATION_RECORD_BYTES);
    let primary = census_report::report::ExportResourceError::BudgetExceeded {
        what: "primary budget",
        needed: 2,
        available: 1,
    };
    let result = ledger::finish(
        &store,
        &key(&request),
        attempt,
        ledger::Attempt::Failed(JobError::Resource {
            source: primary,
            message: "original resource".into(),
        }),
        &[oversized],
    );
    check!(matches!(result, Err(JobError::Resource {
        source: census_report::report::ExportResourceError::BudgetExceeded { what: "primary budget", needed: 2, available: 1 }, message
    }) if message.contains("original resource")));
    Ok(())
}
