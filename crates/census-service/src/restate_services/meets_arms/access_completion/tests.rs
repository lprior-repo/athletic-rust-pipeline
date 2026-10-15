use super::*;
use crate::census::{MeetCensus, MeetSourceRows};
use crate::restate_services::history_stage::HistoricalStageScope;
use crate::restate_services::meets_arms::collect_fragment;
use crate::restate_services::tests::sdk_error;
use crate::restate_services::wire::HistoryWindow;
use census_crawl::{CollectionDisposition, Recording};
use census_domain::model::AccessBlockKind;
use census_store::Table;
use restate_sdk::prelude::Json;
use std::sync::Arc;

mod fixture;
use fixture::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn meet_boundary_retains_actual_refusals_with_exact_owner_and_outcome_on_reopen() -> TestResult
{
    for status in [403, 429] {
        let root = tempfile::tempdir()?;
        let (fetcher, observed) = observed_refusal(root.path(), status).await?;
        let fetcher = Arc::new(fetcher.with_offline(true));
        let identity = format!("retained-access-{status}/0");
        let store = Arc::new(Store::open(root.path().join("store"))?);
        let Json(outcome) = collect_fragment(
            Arc::clone(&store),
            Arc::clone(&fetcher),
            scope()?,
            ("athleticnet".into(), 0),
            identity.clone(),
        )
        .await
        .map_err(sdk_error)?;
        let row = outcome.census.sources.first().ok_or("source outcome")?;
        check!(eq; row.slug, "athleticnet");
        check!(eq; row.disposition, CollectionDisposition::Blocked);
        check!(!outcome.census.is_terminal());
        check!(row
            .notes
            .iter()
            .any(|note| note.contains(diagnostics::PHASE)));
        let expected_outcome = serde_json::to_value(&outcome)?;
        let original = store.journal_payload(diagnostics::PHASE, &identity)?;
        check!(original.is_some());
        check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![observed.clone()]);
        check!(eq; store.journal_payload(OUTCOME_PHASE, &identity)?, Some(expected_outcome.clone()));
        drop(store);
        let store = Store::open(root.path().join("store"))?;
        check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![observed]);
        check!(eq; store.journal_payload(OUTCOME_PHASE, &identity)?, Some(expected_outcome));
        check!(eq; store.journal_payload(diagnostics::PHASE, &identity)?, original);
        check!(eq; fetcher.stats().await.physical_requests(), 1);
    }
    Ok(())
}

#[test]
fn bounded_access_rejection_leaves_original_diagnostics_and_no_completion_or_partial_rows(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let outcome = outcome(&store, "oversized-access/0")?;
    let original = store.journal_payload(diagnostics::PHASE, &outcome.identity)?;
    let valid = condition("first.example", "2026-10-14T11:00:00Z", 429);
    let mut oversized = condition("large.example", "2026-10-14T11:00:00Z", 403);
    oversized.detail = "x".repeat(census_store::MAX_OBSERVATION_RECORD_BYTES);
    let error = persist(&store, &outcome, &[valid, oversized])
        .err()
        .ok_or("oversized access unexpectedly committed")?;
    let detail = retention_failure(&outcome.identity, error.into()).to_string();
    check!(detail.contains("meet_fragment_diagnostics_v1/oversized-access/0"));
    check!(detail.contains("retaining access evidence also failed"));
    check!(store
        .journal_payload(OUTCOME_PHASE, &outcome.identity)?
        .is_none());
    check!(eq; store.journal_payload(diagnostics::PHASE, &outcome.identity)?, original);
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 0);
    Ok(())
}

#[test]
fn rejected_outcome_journal_leaves_staged_access_uncommitted() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let mut outcome = outcome(&store, "journal-rejection/0")?;
    let identity = outcome.identity.clone();
    let original = store.journal_payload(diagnostics::PHASE, &identity)?;
    let sequence = store.snapshot().sequence();
    outcome.identity = "x".repeat(census_store::MAX_JOURNAL_KEY_BYTES);
    let expected_bytes = OUTCOME_PHASE
        .len()
        .checked_add(outcome.identity.len())
        .and_then(|bytes| bytes.checked_add(1))
        .ok_or("journal key byte count")?;
    let valid = condition("first.example", "2026-10-14T11:00:00Z", 429);
    let error = persist(&store, &outcome, &[valid])
        .err()
        .ok_or("oversized outcome journal unexpectedly committed")?;
    check!(matches!(error, census_store::StoreError::JournalTooLarge {
        what: "key", ref phase, ref key, bytes, max
    } if phase == OUTCOME_PHASE && key == &"x".repeat(64)
        && bytes == expected_bytes && max == census_store::MAX_JOURNAL_KEY_BYTES));
    check!(eq; store.journal_keys(OUTCOME_PHASE)?, std::collections::HashSet::<String>::new());
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 0);
    check!(eq; store.journal_payload(diagnostics::PHASE, &identity)?, original);
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn commit_time_access_decode_failure_does_not_publish_a_staged_outcome() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let outcome = outcome(&store, "decode-failure/0")?;
    let expected = condition("broken.example", "2026-10-14T11:00:00Z", 403);
    let malformed = serde_json::json!({"id": expected.id, "malformed_fixture": true});
    store.replace(Table::SourceAccess, &malformed)?;
    let before = store.snapshot().tables_digest(&[Table::SourceAccess])?;
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 1);
    let original = store.journal_payload(diagnostics::PHASE, &outcome.identity)?;
    let sequence = store.snapshot().sequence();
    let valid = condition("first.example", "2026-10-14T11:00:00Z", 429);
    let error = persist(&store, &outcome, &[valid, expected])
        .err()
        .ok_or("malformed retained access unexpectedly committed")?;
    check!(matches!(error, census_store::StoreError::Json { .. }));
    check!(store
        .journal_payload(OUTCOME_PHASE, &outcome.identity)?
        .is_none());
    check!(eq; store.snapshot().tables_digest(&[Table::SourceAccess])?, before);
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 1);
    check!(eq; store.journal_payload(diagnostics::PHASE, &outcome.identity)?, original);
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn older_access_flush_keeps_newest_condition_and_identical_replay_keeps_one_row() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let newer = condition("same.example", "2026-10-14T11:00:00Z", 429);
    let latest = outcome(&store, "newer-access/0")?;
    persist(&store, &latest, std::slice::from_ref(&newer))?;
    let before = store.walk_table(Table::SourceAccess)?;
    check!(eq; before.rows, 1);
    let mut older = condition("same.example", "2026-10-13T11:00:00Z", 429);
    older.detail = "older observed detail".into();
    older.retry_after_seconds = Some(10);
    older.cooldown_until = Some("2026-10-13T12:00:00Z".into());
    let earlier = outcome(&store, "older-access/0")?;
    persist(&store, &earlier, &[older])?;
    persist(&store, &latest, std::slice::from_ref(&newer))?;
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![newer]);
    check!(eq; store.walk_table(Table::SourceAccess)?, before);
    check!(eq; store.journal_payload(OUTCOME_PHASE, &earlier.identity)?, Some(serde_json::to_value(&earlier)?));
    check!(eq; store.journal_payload(OUTCOME_PHASE, &latest.identity)?, Some(serde_json::to_value(&latest)?));
    Ok(())
}

#[tokio::test]
async fn historical_outcome_replay_does_not_backfill_unrelated_current_access() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path().join("store"))?);
    let historical = outcome(&store, "historical-outcome/0")?;
    let payload = serde_json::to_value(&historical)?;
    store.journal_done(OUTCOME_PHASE, &historical.identity, &historical)?;
    let diagnostics = store.journal_payload(diagnostics::PHASE, &historical.identity)?;
    drop(store);
    let store = Arc::new(Store::open(root.path().join("store"))?);
    let (fetcher, _) = observed_refusal(root.path(), 429).await?;
    let fetcher = Arc::new(fetcher.with_offline(true));
    fetcher
        .record_access_condition(
            "oversized.example",
            AccessBlockKind::Forbidden,
            403,
            None,
            "x".repeat(census_store::MAX_OBSERVATION_RECORD_BYTES),
        )
        .await;
    check!(matches!(
        fetcher.access_conditions().await,
        Err(census_crawl::CrawlError::Store(
            census_store::StoreError::RowTooLarge { .. }
        ))
    ));
    let sequence = store.snapshot().sequence();
    let Json(replayed) = collect_fragment(
        Arc::clone(&store),
        Arc::clone(&fetcher),
        scope()?,
        ("wiaa_results".into(), 0),
        historical.identity.clone(),
    )
    .await
    .map_err(sdk_error)?;
    check!(eq; serde_json::to_value(replayed)?, payload);
    check!(eq; store.journal_payload(OUTCOME_PHASE, &historical.identity)?, Some(payload));
    check!(eq; store.journal_payload(diagnostics::PHASE, &historical.identity)?, diagnostics);
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 0);
    check!(eq; fetcher.stats().await.physical_requests(), 1);
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}

#[tokio::test]
async fn oversized_fetcher_snapshot_cannot_acknowledge_meet_completion() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path().join("store"))?);
    let fetcher = Arc::new(fetcher(root.path())?.with_offline(true));
    fetcher
        .record_access_condition(
            "oversized.example",
            AccessBlockKind::Forbidden,
            403,
            None,
            "x".repeat(census_store::MAX_OBSERVATION_RECORD_BYTES),
        )
        .await;
    let identity = "oversized-snapshot/0";
    let error = collect_fragment(
        Arc::clone(&store),
        fetcher,
        scope()?,
        ("athleticnet".into(), 0),
        identity.into(),
    )
    .await
    .err()
    .ok_or("oversized snapshot unexpectedly completed")?;
    let detail = sdk_error(error).to_string();
    check!(detail.contains("meet_fragment_diagnostics_v1/oversized-snapshot/0"));
    check!(detail.contains("retaining access evidence also failed"));
    check!(store.journal_payload(OUTCOME_PHASE, identity)?.is_none());
    let raw = store
        .journal_payload(diagnostics::PHASE, identity)?
        .ok_or("original diagnostics")?;
    let raw: MeetCensus = serde_json::from_value(raw)?;
    let source = raw.sources.first().ok_or("source refusal")?;
    check!(eq; source.disposition, CollectionDisposition::Blocked);
    check!(source
        .notes
        .iter()
        .any(|note| note.contains("prerequisite missing")));
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 0);
    Ok(())
}

#[tokio::test]
async fn static_terms_and_prerequisite_refusals_never_manufacture_http_access_rows() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path().join("store"))?);
    let fetcher = Arc::new(fetcher(root.path())?.with_offline(true));
    for slug in ["tfrrs", "athleticnet"] {
        let identity = format!("static-{slug}/0");
        let Json(outcome) = collect_fragment(
            Arc::clone(&store),
            Arc::clone(&fetcher),
            scope()?,
            (slug.into(), 0),
            identity.clone(),
        )
        .await
        .map_err(sdk_error)?;
        let source = outcome.census.sources.first().ok_or("static refusal")?;
        check!(eq; source.slug, slug);
        check!(eq; source.disposition, CollectionDisposition::Blocked);
        check!(!outcome.census.is_terminal());
        check!(store.journal_payload(OUTCOME_PHASE, &identity)?.is_some());
    }
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 0);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    check!(fetcher.access_conditions().await?.is_empty());
    Ok(())
}

#[test]
fn diagnostic_context_preserves_retention_error_retry_and_resource_classes() -> TestResult {
    let transient = retention_failure(
        "fixture/0",
        JobError::Transient {
            message: "write failure".into(),
        },
    );
    check!(matches!(transient, JobError::Transient { ref message }
        if message.contains("write failure") && message.contains("meet_fragment_diagnostics_v1/fixture/0")));
    let terminal = retention_failure(
        "fixture/0",
        JobError::Terminal {
            message: "invalid row".into(),
        },
    );
    check!(matches!(terminal, JobError::Terminal { ref message }
        if message.contains("invalid row") && message.contains("meet_fragment_diagnostics_v1/fixture/0")));
    let resource = retention_failure(
        "fixture/0",
        JobError::Resource {
            source: census_report::report::ExportResourceError::BudgetExceeded {
                what: "fixture",
                needed: 2,
                available: 1,
            },
            message: "resource failure".into(),
        },
    );
    check!(matches!(resource, JobError::Resource {
        source: census_report::report::ExportResourceError::BudgetExceeded {
            what: "fixture", needed: 2, available: 1,
        }, ref message
    } if message.contains("resource failure") && message.contains("meet_fragment_diagnostics_v1/fixture/0")));
    Ok(())
}
